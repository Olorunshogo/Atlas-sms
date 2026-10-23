use chrono::{Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::{codes, ApiError};

use super::models::{
    CreatePaymentMethodRequest, Payment, PaymentMethod, Refund, UpdatePaymentMethodRequest,
};

/// How long a refund's confirmation token is valid before it must be re-requested - long enough for a cashier to double-check with a manager, short enough that a token left on screen doesn't stay a live "execute this refund" button indefinitely.
const CONFIRMATION_WINDOW_MINUTES: i64 = 15;

pub async fn sale_store_id(conn: &mut PgConnection, sale_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.sales WHERE id = $1")
        .bind(sale_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown sale."));
}

/// The store a payment's own permission check is scoped to - one join away from `sale_store_id`, since a payment only carries `sale_id`, not `store_id` directly.
pub async fn payment_store_id(conn: &mut PgConnection, payment_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar(
        "SELECT s.store_id FROM atlas.payments p JOIN atlas.sales s ON s.id = p.sale_id WHERE p.id = $1",
    )
    .bind(payment_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown payment."));
}

/**
 * The store a refund's own permission check is scoped to.
 * A store-scoped `sale.refund` grant must be checked against this, not `None`/tenant-wide - `assert_permission`'s SQL never matches a store-scope grant when `store_id` is `None`, so passing `None` here would silently exclude every store_manager from confirming a refund at their own store.
 */
pub async fn refund_store_id(conn: &mut PgConnection, refund_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar(
        "SELECT s.store_id FROM atlas.refunds r
         JOIN atlas.sales s ON s.id = r.sale_id
         WHERE r.id = $1",
    )
    .bind(refund_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown refund."));
}

// === Payment methods

const METHOD_COLUMNS: &str = "id, code, name, kind::text AS kind, requires_reference, is_active, sort_order";

/// The default set every new tenant starts with; migration 20261022000032 seeded the same for tenants that already existed.
pub async fn seed_default_methods(conn: &mut PgConnection, tenant_id: Uuid) -> Result<(), ApiError> {
    let defaults = [
        ("cash", "Cash", "cash", false, 1),
        ("card", "Card", "card", false, 2),
        ("transfer", "Bank transfer", "transfer", true, 3),
        ("store_credit", "Store credit", "store_credit", false, 4),
    ];
    for (code, name, kind, requires_reference, sort_order) in defaults {
        sqlx::query(
            "INSERT INTO atlas.payment_methods (id, tenant_id, code, name, kind, requires_reference, sort_order)
             VALUES ($1, $2, $3, $4, $5::atlas.payment_method, $6, $7)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(code)
        .bind(name)
        .bind(kind)
        .bind(requires_reference)
        .bind(sort_order)
        .execute(&mut *conn)
        .await?;
    }
    return Ok(());
}

pub async fn list_methods(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<PaymentMethod>, ApiError> {
    return sqlx::query_as::<_, PaymentMethod>(&format!(
        "SELECT {} FROM atlas.payment_methods WHERE tenant_id = $1 ORDER BY sort_order, name",
        METHOD_COLUMNS
    ))
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_method(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreatePaymentMethodRequest,
) -> Result<PaymentMethod, ApiError> {
    return sqlx::query_as::<_, PaymentMethod>(&format!(
        "INSERT INTO atlas.payment_methods (id, tenant_id, code, name, kind, requires_reference, sort_order)
         VALUES ($1, $2, $3, $4, $5::atlas.payment_method, $6, $7)
         RETURNING {}",
        METHOD_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.code.trim())
    .bind(payload.name.trim())
    .bind(&payload.kind)
    .bind(payload.requires_reference)
    .bind(payload.sort_order)
    .fetch_one(conn)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("A payment method with that code already exists.", codes::PAYMENT_METHOD_EXISTS)
        }
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("22P02") => {
            ApiError::bad_request("kind must be one of cash, card, transfer, wallet, store_credit, other")
        }
        other => ApiError::from(other),
    });
}

pub async fn update_method(
    conn: &mut PgConnection,
    method_id: Uuid,
    payload: &UpdatePaymentMethodRequest,
) -> Result<PaymentMethod, ApiError> {
    return sqlx::query_as::<_, PaymentMethod>(&format!(
        "UPDATE atlas.payment_methods
         SET name = COALESCE($2, name), requires_reference = COALESCE($3, requires_reference),
             is_active = COALESCE($4, is_active), sort_order = COALESCE($5, sort_order)
         WHERE id = $1
         RETURNING {}",
        METHOD_COLUMNS
    ))
    .bind(method_id)
    .bind(&payload.name)
    .bind(payload.requires_reference)
    .bind(payload.is_active)
    .bind(payload.sort_order)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Payment method not found."));
}

/// An active payment method of this tenant (RLS already limits it to the tenant).
pub async fn active_method(conn: &mut PgConnection, method_id: Uuid) -> Result<PaymentMethod, ApiError> {
    return sqlx::query_as::<_, PaymentMethod>(&format!(
        "SELECT {} FROM atlas.payment_methods WHERE id = $1 AND is_active",
        METHOD_COLUMNS
    ))
    .bind(method_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request_with_code("Unknown or inactive payment method.", codes::PAYMENT_METHOD_INVALID));
}

// === Payments

pub const PAYMENT_SELECT: &str = "SELECT p.id, p.sale_id, p.payment_method_id, p.method::text AS method,
            COALESCE(pm.name, p.method::text) AS method_name, p.amount_minor, p.tendered_amount_minor,
            p.change_amount_minor, p.refunded_amount_minor, p.currency, p.status::text AS status,
            p.reference, p.status_reason, p.created_at, p.processed_at
     FROM atlas.payments p
     LEFT JOIN atlas.payment_methods pm ON pm.id = p.payment_method_id";

pub struct NewPayment<'a> {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub sale_id: Uuid,
    pub shift_id: Uuid,
    pub method: &'a PaymentMethod,
    pub amount_minor: i64,
    pub tendered_amount_minor: Option<i64>,
    pub change_amount_minor: i64,
    pub currency: &'a str,
    pub status: &'a str,
    pub reference: Option<&'a str>,
    pub received_by_user_id: Uuid,
}

pub async fn insert_payment(conn: &mut PgConnection, payment: NewPayment<'_>) -> Result<Payment, ApiError> {
    sqlx::query(
        "INSERT INTO atlas.payments
            (id, tenant_id, sale_id, shift_id, payment_method_id, method, amount_minor, tendered_amount_minor,
             change_amount_minor, currency, status, reference, received_by_user_id, processed_at)
         VALUES ($1, $2, $3, $4, $5, $6::atlas.payment_method, $7, $8, $9, $10, $11::atlas.payment_status, $12, $13,
                 CASE WHEN $11 = 'succeeded' THEN now() END)",
    )
    .bind(payment.id)
    .bind(payment.tenant_id)
    .bind(payment.sale_id)
    .bind(payment.shift_id)
    .bind(payment.method.id)
    .bind(&payment.method.kind)
    .bind(payment.amount_minor)
    .bind(payment.tendered_amount_minor)
    .bind(payment.change_amount_minor)
    .bind(payment.currency)
    .bind(payment.status)
    .bind(payment.reference)
    .bind(payment.received_by_user_id)
    .execute(&mut *conn)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.constraint() == Some("payments_reference_unique_idx") => {
            ApiError::conflict_with_code(
                "A live payment with this reference is already recorded.",
                codes::DUPLICATE_PAYMENT_REFERENCE,
            )
        }
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("A payment with this id already exists.", codes::IDEMPOTENCY_CONFLICT)
        }
        other => ApiError::from(other),
    })?;
    return get_payment(conn, payment.id).await;
}

pub async fn get_payment(conn: &mut PgConnection, payment_id: Uuid) -> Result<Payment, ApiError> {
    return sqlx::query_as::<_, Payment>(&format!("{} WHERE p.id = $1", PAYMENT_SELECT))
        .bind(payment_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Payment not found."));
}

pub async fn list_payments_for_sale(
    conn: &mut PgConnection,
    sale_id: Uuid,
) -> Result<Vec<Payment>, ApiError> {
    return sqlx::query_as::<_, Payment>(&format!("{} WHERE p.sale_id = $1 ORDER BY p.created_at, p.id", PAYMENT_SELECT))
        .bind(sale_id)
        .fetch_all(conn)
        .await
        .map_err(ApiError::from);
}

// === Refunds

pub const REFUND_SELECT: &str = "SELECT r.id, r.sale_id, r.return_id, r.payment_id, r.payment_method_id,
            pm.name AS method_name, pm.kind::text AS method_kind, r.amount_minor, r.status::text AS status,
            r.reason, r.reference, r.requested_by_user_id, r.approved_by_user_id, r.approved_at,
            r.executed_at, r.created_at
     FROM atlas.refunds r
     LEFT JOIN atlas.payment_methods pm ON pm.id = r.payment_method_id";

pub async fn get_refund(conn: &mut PgConnection, refund_id: Uuid) -> Result<Refund, ApiError> {
    return sqlx::query_as::<_, Refund>(&format!("{} WHERE r.id = $1", REFUND_SELECT))
        .bind(refund_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Refund not found."));
}

pub async fn list_refunds_for_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<Vec<Refund>, ApiError> {
    return sqlx::query_as::<_, Refund>(&format!("{} WHERE r.sale_id = $1 ORDER BY r.created_at, r.id", REFUND_SELECT))
        .bind(sale_id)
        .fetch_all(conn)
        .await
        .map_err(ApiError::from);
}

/**
 * How much of a sale can still be refunded: what was paid, less what has gone back already, less step-up refunds still waiting on their confirmation token (so two requests can't both claim the same money).
 * Expects the sale row to be locked by the caller.
 */
pub async fn refundable_on_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<i64, ApiError> {
    return sqlx::query_scalar(
        "SELECT s.amount_paid_minor - s.amount_refunded_minor
                - COALESCE((SELECT SUM(r.amount_minor) FROM atlas.refunds r
                            WHERE r.sale_id = s.id AND r.status = 'awaiting_confirmation'
                              AND r.confirmation_expires_at > now()), 0)::bigint
         FROM atlas.sales s WHERE s.id = $1",
    )
    .bind(sale_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * Moves `amount_minor` of a payment into refunded, in one guarded UPDATE: the payment must be succeeded (or already partly refunded) and can never go past its own amount.
 * The single place payment-level refund caps are enforced, for both return refunds and step-up refunds.
 */
pub async fn apply_refund_to_payment(conn: &mut PgConnection, payment_id: Uuid, amount_minor: i64) -> Result<(), ApiError> {
    let updated = sqlx::query(
        "UPDATE atlas.payments
         SET refunded_amount_minor = refunded_amount_minor + $2,
             status = CASE WHEN refunded_amount_minor + $2 = amount_minor
                           THEN 'refunded'::atlas.payment_status
                           ELSE 'partially_refunded'::atlas.payment_status END,
             updated_at = now()
         WHERE id = $1
           AND status IN ('succeeded', 'partially_refunded')
           AND refunded_amount_minor + $2 <= amount_minor",
    )
    .bind(payment_id)
    .bind(amount_minor)
    .execute(conn)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::conflict_with_code(
            "That refund is more than is left on the payment.",
            codes::REFUND_AMOUNT_EXCEEDED,
        ));
    }
    return Ok(());
}

pub async fn open_shift_at(conn: &mut PgConnection, store_id: Uuid) -> Result<Option<Uuid>, ApiError> {
    return sqlx::query_scalar("SELECT id FROM atlas.shifts WHERE store_id = $1 AND status = 'open'")
        .bind(store_id)
        .fetch_optional(conn)
        .await
        .map_err(ApiError::from);
}

/// The tenant's store-credit payment method (seeded for every tenant), which store-credit refunds are recorded against.
pub async fn store_credit_method(conn: &mut PgConnection) -> Result<PaymentMethod, ApiError> {
    return sqlx::query_as::<_, PaymentMethod>(&format!(
        "SELECT {} FROM atlas.payment_methods WHERE kind = 'store_credit' ORDER BY is_active DESC, sort_order LIMIT 1",
        METHOD_COLUMNS
    ))
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request_with_code("No store credit payment method is configured.", codes::PAYMENT_METHOD_INVALID));
}

/**
 * First half of the step-up refund - a refund against one payment that isn't tied to returned goods (a goodwill price adjustment). Mints and stores a hashed confirmation token and returns the raw one exactly once; executes nothing yet.
 * The amount is reserved against both the payment and the sale straight away, so two requests can't each claim the same money before either is confirmed.
 */
pub async fn request_refund(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    reason: &str,
    requested_by_user_id: Uuid,
) -> Result<(Refund, String), ApiError> {
    if amount_minor <= 0 {
        return Err(ApiError::bad_request("refund amount must be positive"));
    }
    if reason.trim().is_empty() {
        return Err(ApiError::bad_request("a refund needs a reason"));
    }
    let payment = get_payment(conn, payment_id).await?;
    crate::sales::service::lock_sale(conn, payment.sale_id).await?;

    let reserved_on_payment: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minor), 0)::bigint FROM atlas.refunds
         WHERE payment_id = $1 AND status = 'awaiting_confirmation' AND confirmation_expires_at > now()",
    )
    .bind(payment_id)
    .fetch_one(&mut *conn)
    .await?;
    let payment_left = if payment.status == "succeeded" || payment.status == "partially_refunded" {
        payment.amount_minor - payment.refunded_amount_minor - reserved_on_payment
    } else {
        0
    };
    let sale_left = refundable_on_sale(conn, payment.sale_id).await?;
    if amount_minor > payment_left.min(sale_left) {
        return Err(ApiError::conflict_with_code(
            format!("Only {} can still be refunded on this payment.", payment_left.min(sale_left).max(0)),
            codes::REFUND_AMOUNT_EXCEEDED,
        ));
    }

    let token = crypto::generate_opaque_token();
    let token_hash = crypto::hash_lookup_token(&token);
    let expires_at = Utc::now() + Duration::minutes(CONFIRMATION_WINDOW_MINUTES);
    let refund_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.refunds
            (id, tenant_id, sale_id, payment_id, payment_method_id, amount_minor, reason,
             confirmation_token_hash, confirmation_expires_at, requested_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(refund_id)
    .bind(tenant_id)
    .bind(payment.sale_id)
    .bind(payment_id)
    .bind(payment.payment_method_id)
    .bind(amount_minor)
    .bind(reason.trim())
    .bind(&token_hash)
    .bind(expires_at)
    .bind(requested_by_user_id)
    .execute(&mut *conn)
    .await?;

    return Ok((get_refund(conn, refund_id).await?, token));
}

/**
 * Second half - executes a refund that's still `awaiting_confirmation`, whose token matches and whose window hasn't expired, then moves the money: the payment's refunded amount and status, and the sale's refunded total.
 * Any other state (already executed, wrong token, expired) is a flat rejection, not a partial success.
 */
pub async fn confirm_refund(
    conn: &mut PgConnection,
    refund_id: Uuid,
    confirmation_token: &str,
    confirmed_by_user_id: Uuid,
) -> Result<Refund, ApiError> {
    let token_hash = crypto::hash_lookup_token(confirmation_token);
    let (sale_id, store_id): (Uuid, Uuid) = sqlx::query_as(
        "SELECT r.sale_id, s.store_id FROM atlas.refunds r JOIN atlas.sales s ON s.id = r.sale_id WHERE r.id = $1",
    )
    .bind(refund_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Invalid, expired, or already-resolved refund confirmation."))?;
    crate::sales::service::lock_sale(conn, sale_id).await?;
    let shift_id = open_shift_at(conn, store_id).await?;

    let executed: Option<(Option<Uuid>, i64)> = sqlx::query_as(
        "UPDATE atlas.refunds
         SET status = 'executed', executed_at = now(), executed_by_user_id = $3,
             approved_by_user_id = $3, approved_at = now(), shift_id = $4
         WHERE id = $1
           AND status = 'awaiting_confirmation'
           AND confirmation_token_hash = $2
           AND confirmation_expires_at > now()
         RETURNING payment_id, amount_minor",
    )
    .bind(refund_id)
    .bind(&token_hash)
    .bind(confirmed_by_user_id)
    .bind(shift_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((payment_id, amount_minor)) = executed else {
        return Err(ApiError::bad_request("Invalid, expired, or already-resolved refund confirmation."));
    };

    if let Some(payment_id) = payment_id {
        apply_refund_to_payment(conn, payment_id, amount_minor).await?;
    }
    sqlx::query("UPDATE atlas.sales SET amount_refunded_minor = amount_refunded_minor + $2 WHERE id = $1")
        .bind(sale_id)
        .bind(amount_minor)
        .execute(&mut *conn)
        .await?;
    return get_refund(conn, refund_id).await;
}
