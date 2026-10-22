/*!
 * Checkout: turns a cart plus its payments into a sale in one transaction.
 * The live `POST /sales` path and the offline `POST /sync/sales` path both
 * come through `create_sale`; `CheckoutMode` is the only difference, and it
 * decides what happens to a policy problem - live refuses, sync records and
 * accepts, because an offline sale has already happened.
 */
use chrono::Utc;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::crypto;
use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::payments::models::PaymentMethod;
use crate::payments::service::{self as payments, NewPayment};
use crate::pricing::engine;
use crate::pricing::models::{Quote, StorePricing, Violation};
use crate::pricing::quote::{self, QuoteContext};
use crate::pricing::resolver;

use super::models::{CreateSaleRequest, PaymentInputStatus, Sale, SaleDetail};
use super::service;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckoutMode {
    /// A cashier ringing the sale up now: every rule is enforced.
    Live,
    /// A device catching up after being offline: the sale already happened, so policy problems are recorded, not refused.
    Sync,
}

/// What the payments decide about the sale.
struct Settlement {
    status: &'static str,
    payment_status: &'static str,
    amount_paid_minor: i64,
    rounding_adjustment_minor: i64,
}

/**
 * Idempotent by the client-generated `payload.id`: a replay of the same body returns the sale already recorded, unchanged; the same id with a different body is `IDEMPOTENCY_CONFLICT`.
 * A per-sale advisory lock makes two simultaneous copies of one request (a retry racing the original) queue up instead of both trying to insert.
 */
pub async fn create_sale(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    cashier_user_id: Uuid,
    payload: &CreateSaleRequest,
    mode: CheckoutMode,
) -> Result<SaleDetail, ApiError> {
    let request_hash = crypto::hash_lookup_token(&serde_json::to_string(payload).map_err(|e| ApiError::Internal(e.to_string()))?);

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("sale:{}", payload.id))
        .execute(&mut *conn)
        .await?;
    if let Some(existing) = service::find_sale(conn, payload.id).await? {
        if existing.request_hash.as_deref().is_some_and(|h| h != request_hash) {
            return Err(ApiError::conflict_with_code(
                "A different sale was already recorded with this id.",
                codes::IDEMPOTENCY_CONFLICT,
            ));
        }
        return service::sale_detail(conn, existing).await;
    }

    if payload.device_id.trim().is_empty() {
        return Err(ApiError::bad_request("device_id is required"));
    }
    check_shift(conn, payload, mode).await?;

    let store = resolver::load_store_pricing(conn, payload.store_id).await?;
    let ctx = QuoteContext {
        tenant_id,
        store_id: payload.store_id,
        user_id: cashier_user_id,
        customer_id: payload.customer_id,
        at: match mode {
            CheckoutMode::Live => Utc::now(),
            CheckoutMode::Sync => payload.client_created_at.unwrap_or_else(Utc::now),
        },
    };
    let priced = quote::build(conn, &ctx, &store, &payload.lines, payload.cart_discount.as_ref()).await?;

    if mode == CheckoutMode::Live {
        // Stock is checked for real, under a lock, by the inventory service;
        // the quote's stock warning is only advisory.
        if let Some(violation) = priced.violations.iter().find(|v| v.code != codes::PRODUCT_OUT_OF_STOCK) {
            return Err(ApiError::forbidden_with_code(violation.message.clone(), violation.code));
        }
        if let Some(expected) = payload.expected_total_amount_minor {
            if expected != priced.total_amount_minor {
                return Err(ApiError::conflict_with_code(
                    format!(
                        "The total has changed: the till showed {}, the server prices this cart at {}.",
                        expected, priced.total_amount_minor
                    ),
                    codes::TOTAL_MISMATCH,
                ));
            }
        }
    }

    let methods = load_methods(conn, payload).await?;
    let settlement = settle(payload, &priced, &store, &methods)?;

    let sale = insert_sale(conn, tenant_id, cashier_user_id, payload, &priced, &settlement, &request_hash).await?;
    insert_lines(conn, tenant_id, sale.id, &priced).await?;

    for (input, method) in payload.payments.iter().zip(&methods) {
        let change = match input.tendered_amount_minor {
            Some(tendered) if method.kind == "cash" => tendered - input.amount_minor,
            _ => 0,
        };
        let payment = payments::insert_payment(
            conn,
            NewPayment {
                id: input.id.unwrap_or_else(Uuid::now_v7),
                tenant_id,
                sale_id: sale.id,
                shift_id: sale.shift_id,
                method,
                amount_minor: input.amount_minor,
                tendered_amount_minor: input.tendered_amount_minor,
                change_amount_minor: change,
                currency: &sale.currency,
                status: match input.status {
                    PaymentInputStatus::Succeeded => "succeeded",
                    PaymentInputStatus::Pending => "pending",
                },
                reference: input.reference.as_deref().map(str::trim).filter(|r| !r.is_empty()),
                received_by_user_id: cashier_user_id,
            },
        )
        .await?;
        audit::record_change(
            conn,
            audit::Change {
                tenant_id,
                actor_user_id: cashier_user_id,
                device_id: Some(&payload.device_id),
                entity_type: "payment",
                entity_id: payment.id,
                action: "payment.record",
                before: None,
                after: serde_json::to_value(&payment).ok(),
            },
        )
        .await?;
    }

    if settlement.status == "completed" {
        let policy = match mode {
            CheckoutMode::Sync => NegativeStockPolicy::AllowAndRecordException,
            CheckoutMode::Live if store.allow_negative_stock => NegativeStockPolicy::Allow,
            CheckoutMode::Live => NegativeStockPolicy::Reject,
        };
        complete_sale(conn, tenant_id, sale.id, cashier_user_id, policy).await?;
    }

    // Written after the idempotency check above - a replay returns early and
    // never reaches this, so a sale created once is audited once.
    let completed = service::get_sale(conn, sale.id).await?;
    let detail = service::sale_detail(conn, completed).await?;
    audit::record_change(
        conn,
        audit::Change {
            tenant_id,
            actor_user_id: cashier_user_id,
            device_id: Some(&payload.device_id),
            entity_type: "sale",
            entity_id: sale.id,
            action: "sale.create",
            before: None,
            after: serde_json::to_value(&detail.sale).ok(),
        },
    )
    .await?;
    if mode == CheckoutMode::Sync && !priced.violations.is_empty() {
        record_sync_violations(conn, tenant_id, cashier_user_id, payload, &priced.violations).await?;
    }

    return Ok(detail);
}

/**
 * Moves a sale to `completed`: deducts its stock through the inventory service (in base units, against the sale as `reference_id`) and stamps `completed_at`.
 * Shared by checkout and by a pending payment that later succeeds.
 */
pub async fn complete_sale(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    sale_id: Uuid,
    actor_user_id: Uuid,
    policy: NegativeStockPolicy,
) -> Result<(), ApiError> {
    let sale = service::get_sale(conn, sale_id).await?;
    let lines = service::list_sale_lines(conn, sale_id).await?;
    let posting_lines: Vec<PostingLine> = lines
        .iter()
        .map(|line| PostingLine {
            product_id: line.product_id,
            stock_item_id: line.stock_item_id,
            unit_id: Some(line.unit_id),
            quantity: line.quantity,
        })
        .collect();
    inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: sale.store_id,
            movement_type: MovementType::Sale,
            reason: "Sale",
            reference_id: Some(sale_id),
            actor_user_id,
            policy,
            source: if policy == NegativeStockPolicy::AllowAndRecordException { "offline_sale" } else { "sale" },
            lines: &posting_lines,
        },
    )
    .await?;

    sqlx::query("UPDATE atlas.sales SET status = 'completed', completed_at = now() WHERE id = $1")
        .bind(sale_id)
        .execute(&mut *conn)
        .await?;
    return Ok(());
}

/// The shift must belong to the sale's store. A live sale also needs it open; a synced sale may land after its shift closed, since it was rung up while the shift was open.
async fn check_shift(conn: &mut PgConnection, payload: &CreateSaleRequest, mode: CheckoutMode) -> Result<(), ApiError> {
    let shift: Option<(Uuid, String)> = sqlx::query_as("SELECT store_id, status::text FROM atlas.shifts WHERE id = $1")
        .bind(payload.shift_id)
        .fetch_optional(conn)
        .await?;
    return match shift {
        Some((store_id, _)) if store_id != payload.store_id => Err(ApiError::bad_request_with_code(
            "That shift belongs to a different store.",
            codes::SHIFT_NOT_OPEN,
        )),
        Some((_, status)) if status != "open" && mode == CheckoutMode::Live => {
            Err(ApiError::conflict_with_code("That shift is closed. Open a new shift to sell.", codes::SHIFT_NOT_OPEN))
        }
        Some(_) => Ok(()),
        None => Err(ApiError::bad_request_with_code("Unknown shift.", codes::SHIFT_NOT_OPEN)),
    };
}

async fn load_methods(conn: &mut PgConnection, payload: &CreateSaleRequest) -> Result<Vec<PaymentMethod>, ApiError> {
    let mut methods = Vec::with_capacity(payload.payments.len());
    for input in &payload.payments {
        methods.push(payments::active_method(conn, input.payment_method_id).await?);
    }
    return Ok(methods);
}

/**
 * Checks every payment and decides the sale's outcome (FR-16).
 * Successful payments must cover the amount due exactly - no more (change belongs on the cash row as `tendered - amount`, never as overpayment) and no less unless pending payments make up the rest (`awaiting_payment`) or the store allows pay-later for a named customer. Cash rounding applies only when cash is part of the tender.
 */
fn settle(
    payload: &CreateSaleRequest,
    priced: &Quote,
    store: &StorePricing,
    methods: &[PaymentMethod],
) -> Result<Settlement, ApiError> {
    let mut succeeded = 0i64;
    let mut pending = 0i64;
    let mut has_cash = false;

    for (input, method) in payload.payments.iter().zip(methods) {
        if input.amount_minor <= 0 {
            return Err(ApiError::bad_request("payment amounts must be positive"));
        }
        if method.kind == "store_credit" {
            return Err(ApiError::bad_request_with_code(
                "Store credit is spent through a refund to store credit, not as a checkout tender yet.",
                codes::PAYMENT_METHOD_INVALID,
            ));
        }
        if method.requires_reference && input.reference.as_deref().is_none_or(|r| r.trim().is_empty()) {
            return Err(ApiError::bad_request_with_code(
                format!("{} payments need a transaction reference.", method.name),
                codes::PAYMENT_REFERENCE_REQUIRED,
            ));
        }
        if let Some(tendered) = input.tendered_amount_minor {
            if method.kind != "cash" && tendered != input.amount_minor {
                return Err(ApiError::bad_request("only cash payments can be tendered above the amount applied"));
            }
            if tendered < input.amount_minor {
                return Err(ApiError::bad_request("tendered amount is less than the amount applied"));
            }
        }
        if method.kind == "cash" {
            has_cash = true;
            if input.status == PaymentInputStatus::Pending {
                return Err(ApiError::bad_request("a cash payment can't be pending"));
            }
        }
        match input.status {
            PaymentInputStatus::Succeeded => succeeded += input.amount_minor,
            PaymentInputStatus::Pending => pending += input.amount_minor,
        }
    }

    let rounding = if has_cash {
        engine::cash_rounding_adjustment(priced.total_amount_minor, store.cash_rounding_increment_minor)
    } else {
        0
    };
    let due = priced.total_amount_minor + rounding;

    if succeeded + pending > due {
        return Err(ApiError::bad_request_with_code(
            format!("Payments total {} but only {} is due. Record cash change as tendered, not as extra payment.", succeeded + pending, due),
            codes::PAYMENT_OVERPAID,
        ));
    }

    let (status, payment_status) = if succeeded == due {
        ("completed", "paid")
    } else if pending > 0 && succeeded + pending == due {
        ("awaiting_payment", if succeeded > 0 { "partially_paid" } else { "unpaid" })
    } else if store_allows_pay_later(store, payload) {
        ("completed", if succeeded > 0 { "partially_paid" } else { "unpaid" })
    } else {
        return Err(ApiError::bad_request_with_code(
            format!("Payments of {} don't cover the {} due.", succeeded, due),
            codes::PAYMENT_INSUFFICIENT,
        ));
    };

    return Ok(Settlement {
        status,
        payment_status,
        amount_paid_minor: succeeded,
        rounding_adjustment_minor: rounding,
    });
}

fn store_allows_pay_later(store: &StorePricing, payload: &CreateSaleRequest) -> bool {
    return store.allow_pay_later && payload.customer_id.is_some();
}

async fn insert_sale(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    cashier_user_id: Uuid,
    payload: &CreateSaleRequest,
    priced: &Quote,
    settlement: &Settlement,
    request_hash: &str,
) -> Result<Sale, ApiError> {
    sqlx::query(
        "INSERT INTO atlas.sales
            (id, tenant_id, store_id, shift_id, customer_id, cashier_user_id, device_id, status, currency,
             subtotal_amount_minor, discount_amount_minor, cart_discount_amount_minor, discount_reason,
             tax_amount_minor, total_amount_minor, rounding_adjustment_minor, amount_paid_minor,
             payment_status, prices_include_tax, price_overridden, client_created_at, request_hash, synced_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::atlas.sale_status, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                 $18::atlas.sale_payment_status, $19, $20, $21, $22, now())",
    )
    .bind(payload.id)
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(payload.shift_id)
    .bind(payload.customer_id)
    .bind(cashier_user_id)
    .bind(payload.device_id.trim())
    // A sale headed for `completed` is written as awaiting payment first;
    // `complete_sale` flips it only once its stock has actually been taken.
    .bind(if settlement.status == "completed" { "awaiting_payment" } else { settlement.status })
    .bind(&priced.currency)
    .bind(priced.subtotal_amount_minor)
    .bind(priced.discount_amount_minor)
    .bind(priced.cart_discount_amount_minor)
    .bind(&priced.discount_reason)
    .bind(priced.tax_amount_minor)
    .bind(priced.total_amount_minor)
    .bind(settlement.rounding_adjustment_minor)
    .bind(settlement.amount_paid_minor)
    .bind(settlement.payment_status)
    .bind(priced.prices_include_tax)
    .bind(priced.price_overridden)
    .bind(payload.client_created_at)
    .bind(request_hash)
    .execute(&mut *conn)
    .await
    .map_err(|err| match err {
        // Same id already used - by another tenant (invisible here through
        // RLS) or a twin that slipped past the lock. Either way it isn't this
        // request's sale to return.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("A different sale was already recorded with this id.", codes::IDEMPOTENCY_CONFLICT)
        }
        other => ApiError::from(other),
    })?;
    return service::get_sale(conn, payload.id).await;
}

async fn insert_lines(conn: &mut PgConnection, tenant_id: Uuid, sale_id: Uuid, priced: &Quote) -> Result<(), ApiError> {
    for line in &priced.lines {
        sqlx::query(
            "INSERT INTO atlas.sale_lines
                (id, tenant_id, sale_id, line_number, product_id, product_name, sku, stock_item_id, unit_id, unit_name,
                 quantity, base_quantity, list_unit_price_amount_minor, unit_price_amount_minor, price_rule_id,
                 price_overridden, price_override_reason, line_discount_amount_minor,
                 allocated_cart_discount_amount_minor, discount_reason, net_amount_minor, tax_rate_id,
                 tax_rate_basis_points, tax_amount_minor, line_total_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                     $21, $22, $23, $24, $25)",
        )
        .bind(line.client_line_id.unwrap_or_else(Uuid::now_v7))
        .bind(tenant_id)
        .bind(sale_id)
        .bind(line.line_number)
        .bind(line.product_id)
        .bind(&line.product_name)
        .bind(&line.sku)
        .bind(line.stock_item_id)
        .bind(line.unit_id)
        .bind(&line.unit_name)
        .bind(line.quantity)
        .bind(line.base_quantity)
        .bind(line.list_unit_price_amount_minor)
        .bind(line.unit_price_amount_minor)
        .bind(line.price_rule_id)
        .bind(line.price_overridden)
        .bind(&line.price_override_reason)
        .bind(line.line_discount_amount_minor)
        .bind(line.allocated_cart_discount_amount_minor)
        .bind(&line.discount_reason)
        .bind(line.net_amount_minor)
        .bind(line.tax_rate_id)
        .bind(line.tax_rate_basis_points as i32)
        .bind(line.tax_amount_minor)
        .bind(line.line_total_amount_minor)
        .execute(&mut *conn)
        .await?;
    }
    return Ok(());
}

/// An offline sale that broke a rule a live sale would have been refused for is accepted, but leaves an audit trail a manager can review.
async fn record_sync_violations(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    payload: &CreateSaleRequest,
    violations: &[Violation],
) -> Result<(), ApiError> {
    let drift: Vec<_> = payload
        .lines
        .iter()
        .filter(|line| line.unit_price_amount_minor.is_some())
        .map(|line| json!({ "product_id": line.product_id, "device_price_minor": line.unit_price_amount_minor }))
        .collect();
    return audit::record_change(
        conn,
        audit::Change {
            tenant_id,
            actor_user_id,
            device_id: Some(&payload.device_id),
            entity_type: "sale",
            entity_id: payload.id,
            action: "sale.sync_policy_exception",
            before: None,
            after: Some(json!({ "violations": violations, "device_prices": drift })),
        },
    )
    .await;
}
