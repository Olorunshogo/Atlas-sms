use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory;
use crate::roles;

use super::engine::{self, LineInput};
use super::models::{CartLineInput, DiscountInput, PricedLine, Quote, StorePricing, Violation};
use super::resolver;

/// Who is pricing the cart, where, and as of when. `at` is the device's sale time for an offline sale, so it's priced by the rules that applied when it happened.
pub struct QuoteContext {
    pub tenant_id: Uuid,
    pub store_id: Uuid,
    pub user_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    name: String,
    sku: Option<String>,
    is_active: bool,
    tax_rate_id: Option<Uuid>,
    is_returnable: bool,
    identity_mode: String,
    listed: Option<bool>,
}

/**
 * Prices a cart entirely on the server: resolves each line's price from `price_rules`, its tax rate, its base-unit quantity, then runs the engine.
 * Structural problems (unknown product, wrong unit, missing batch/serial) fail outright. Policy problems (price override or discount beyond the caller's permissions, short stock) come back as `violations`, because what to do about them depends on the caller - a quote shows them, live checkout refuses, offline sync accepts and records them.
 */
pub async fn build(
    conn: &mut PgConnection,
    ctx: &QuoteContext,
    store: &StorePricing,
    lines: &[CartLineInput],
    cart_discount: Option<&DiscountInput>,
) -> Result<Quote, ApiError> {
    if lines.is_empty() {
        return Err(ApiError::bad_request("a sale must have at least one line"));
    }
    if !store.is_active {
        return Err(ApiError::bad_request_with_code("This store is not active.", codes::STORE_INACTIVE));
    }

    let customer_group_id = match ctx.customer_id {
        Some(customer_id) => customer_group(conn, customer_id).await?,
        None => None,
    };

    let mut violations = Vec::new();
    let mut priced = Vec::with_capacity(lines.len());
    let mut engine_lines = Vec::with_capacity(lines.len());

    for (index, input) in lines.iter().enumerate() {
        let line_number = index as i32 + 1;
        if input.quantity <= Decimal::ZERO {
            return Err(ApiError::bad_request(format!("line {}: quantity must be positive", line_number)));
        }

        let product = load_product(conn, input.product_id, ctx.store_id).await?;
        if !product.is_active || product.listed != Some(true) {
            return Err(ApiError::bad_request_with_code(
                format!("line {}: {} is not sold at this store.", line_number, product.name),
                codes::PRODUCT_NOT_SOLD_HERE,
            ));
        }
        check_stock_item(conn, input, &product, line_number).await?;

        let factor = inventory::service::unit_factor(conn, input.product_id, input.unit_id).await?;
        let base_quantity = input.quantity * factor;
        let unit_name: String = sqlx::query_scalar("SELECT name FROM atlas.units_of_measure WHERE id = $1")
            .bind(input.unit_id)
            .fetch_one(&mut *conn)
            .await?;

        let resolved = resolver::resolve_base_price(
            conn,
            input.product_id,
            ctx.store_id,
            customer_group_id,
            base_quantity,
            ctx.at,
        )
        .await?;
        let list_price = resolved.as_ref().map(|r| engine::round_minor(Decimal::from(r.base_unit_price_minor) * factor));

        let (unit_price, overridden) = match (list_price, input.unit_price_amount_minor) {
            (_, Some(client)) if client < 0 => {
                return Err(ApiError::bad_request(format!("line {}: price must not be negative", line_number)));
            }
            (Some(list), Some(client)) => (client, client != list),
            (Some(list), None) => (list, false),
            (None, Some(client)) => (client, true),
            (None, None) => {
                return Err(ApiError::conflict_with_code(
                    format!("line {}: {} has no price at this store.", line_number, product.name),
                    codes::PRODUCT_NOT_PRICED,
                ));
            }
        };
        if overridden {
            let reason_given = input.price_override_reason.as_deref().is_some_and(|r| !r.trim().is_empty());
            let allowed = roles::service::has_permission(conn, ctx.user_id, ctx.tenant_id, Some(ctx.store_id), "sale.price_override").await?;
            if !allowed || !reason_given {
                violations.push(Violation {
                    code: codes::PRICE_OVERRIDE_FORBIDDEN,
                    message: if allowed {
                        format!("line {}: a price override needs a reason.", line_number)
                    } else {
                        format!("line {}: changing the price needs the sale.price_override permission.", line_number)
                    },
                    line_number: Some(line_number),
                });
            }
        }

        let (tax_rate_id, tax_bp) = resolver::effective_tax_rate(conn, product.tax_rate_id, store).await?;
        let line_discount = match &input.discount {
            Some(discount) => discount.to_engine()?,
            None => None,
        };
        if line_discount.is_some() && !has_reason(input.discount.as_ref()) {
            return Err(ApiError::bad_request(format!("line {}: a discount needs a reason", line_number)));
        }

        engine_lines.push(LineInput {
            unit_price_minor: unit_price,
            quantity: input.quantity,
            discount: line_discount,
            tax_rate_basis_points: tax_bp,
        });
        priced.push(PricedLine {
            line_number,
            client_line_id: input.id,
            product_id: input.product_id,
            product_name: product.name,
            sku: product.sku,
            is_returnable: product.is_returnable,
            stock_item_id: input.stock_item_id,
            unit_id: input.unit_id,
            unit_name,
            quantity: input.quantity,
            base_quantity,
            list_unit_price_amount_minor: list_price,
            unit_price_amount_minor: unit_price,
            price_rule_id: resolved.map(|r| r.price_rule_id),
            price_overridden: overridden,
            price_override_reason: input.price_override_reason.clone().filter(|_| overridden),
            gross_amount_minor: 0,
            line_discount_amount_minor: 0,
            allocated_cart_discount_amount_minor: 0,
            net_amount_minor: 0,
            tax_rate_id,
            tax_rate_basis_points: tax_bp,
            tax_amount_minor: 0,
            line_total_amount_minor: 0,
            on_hand: Decimal::ZERO,
            discount_reason: input.discount.as_ref().and_then(|d| d.reason.clone()),
        });
    }

    let cart = match cart_discount {
        Some(discount) => discount.to_engine()?,
        None => None,
    };
    if cart.is_some() && !has_reason(cart_discount) {
        return Err(ApiError::bad_request("a cart discount needs a reason"));
    }

    let totals = engine::price_cart(&engine_lines, cart, store.prices_include_tax);
    for (line, line_totals) in priced.iter_mut().zip(&totals.lines) {
        line.gross_amount_minor = line_totals.gross_minor;
        line.line_discount_amount_minor = line_totals.line_discount_minor;
        line.allocated_cart_discount_amount_minor = line_totals.allocated_cart_discount_minor;
        line.net_amount_minor = line_totals.net_minor;
        line.tax_amount_minor = line_totals.tax_minor;
        line.line_total_amount_minor = line_totals.total_minor;
    }

    // Discount ceiling: anything above the store's cashier allowance (as a
    // share of gross) needs `sale.discount`.
    if totals.discount_minor > 0 {
        let allowance = Decimal::from(totals.subtotal_minor) * Decimal::from(store.max_cashier_discount_basis_points)
            / Decimal::from(engine::BASIS_POINTS);
        if Decimal::from(totals.discount_minor) > allowance
            && !roles::service::has_permission(conn, ctx.user_id, ctx.tenant_id, Some(ctx.store_id), "sale.discount").await?
        {
            violations.push(Violation {
                code: codes::DISCOUNT_FORBIDDEN,
                message: "This discount is above the cashier limit and needs the sale.discount permission.".to_string(),
                line_number: None,
            });
        }
    }

    // Informational only - the inventory service makes the binding check
    // under a lock at checkout.
    let mut needed: BTreeMap<(Uuid, Option<Uuid>), Decimal> = BTreeMap::new();
    for line in &priced {
        *needed.entry((line.product_id, line.stock_item_id)).or_default() += line.base_quantity;
    }
    let mut on_hand_by_key = BTreeMap::new();
    for ((product_id, stock_item_id), required) in &needed {
        let available = inventory::service::on_hand(conn, ctx.store_id, *product_id, *stock_item_id).await?;
        on_hand_by_key.insert((*product_id, *stock_item_id), available);
        if available < *required && !store.allow_negative_stock {
            violations.push(Violation {
                code: codes::PRODUCT_OUT_OF_STOCK,
                message: format!(
                    "Not enough stock for product {}: requested {}, available {}.",
                    product_id,
                    required.normalize(),
                    available.normalize()
                ),
                line_number: priced
                    .iter()
                    .find(|l| l.product_id == *product_id && l.stock_item_id == *stock_item_id)
                    .map(|l| l.line_number),
            });
        }
    }
    for line in priced.iter_mut() {
        line.on_hand = on_hand_by_key[&(line.product_id, line.stock_item_id)];
    }

    let cash_total = totals.total_minor + engine::cash_rounding_adjustment(totals.total_minor, store.cash_rounding_increment_minor);

    return Ok(Quote {
        store_id: ctx.store_id,
        currency: store.currency.clone(),
        prices_include_tax: store.prices_include_tax,
        customer_id: ctx.customer_id,
        customer_group_id,
        price_overridden: priced.iter().any(|l| l.price_overridden),
        lines: priced,
        subtotal_amount_minor: totals.subtotal_minor,
        discount_amount_minor: totals.discount_minor,
        cart_discount_amount_minor: totals.cart_discount_minor,
        discount_reason: cart_discount.and_then(|d| d.reason.clone()),
        tax_amount_minor: totals.tax_minor,
        total_amount_minor: totals.total_minor,
        cash_total_amount_minor: cash_total,
        cash_rounding_increment_minor: store.cash_rounding_increment_minor,
        priced_at: ctx.at,
        violations,
    });
}

fn has_reason(discount: Option<&DiscountInput>) -> bool {
    return discount
        .and_then(|d| d.reason.as_deref())
        .is_some_and(|r| !r.trim().is_empty());
}

async fn customer_group(conn: &mut PgConnection, customer_id: Uuid) -> Result<Option<Uuid>, ApiError> {
    let row: Option<Option<Uuid>> =
        sqlx::query_scalar("SELECT customer_group_id FROM atlas.customers WHERE id = $1 AND deleted_at IS NULL")
            .bind(customer_id)
            .fetch_optional(conn)
            .await?;
    return row.ok_or_else(|| ApiError::bad_request("Unknown customer."));
}

async fn load_product(conn: &mut PgConnection, product_id: Uuid, store_id: Uuid) -> Result<ProductRow, ApiError> {
    return sqlx::query_as::<_, ProductRow>(
        "SELECT p.name, p.sku, p.is_active, p.tax_rate_id, p.is_returnable,
                pt.identity_mode::text AS identity_mode, psl.is_active AS listed
         FROM atlas.products p
         JOIN atlas.product_types pt ON pt.id = p.product_type_id
         LEFT JOIN atlas.product_store_listings psl ON psl.product_id = p.id AND psl.store_id = $2
         WHERE p.id = $1 AND p.deleted_at IS NULL",
    )
    .bind(product_id)
    .bind(store_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request_with_code("Unknown product.", codes::PRODUCT_NOT_SOLD_HERE));
}

/// Batch- and serial-tracked products must name the stock item being sold, and a serial number is always exactly one unit.
async fn check_stock_item(
    conn: &mut PgConnection,
    input: &CartLineInput,
    product: &ProductRow,
    line_number: i32,
) -> Result<(), ApiError> {
    if product.identity_mode == "by_product" {
        return Ok(());
    }
    let Some(stock_item_id) = input.stock_item_id else {
        return Err(ApiError::bad_request_with_code(
            format!("line {}: {} needs a batch or serial number.", line_number, product.name),
            codes::STOCK_ITEM_REQUIRED,
        ));
    };
    let serial: Option<Option<String>> =
        sqlx::query_scalar("SELECT serial_number FROM atlas.stock_items WHERE id = $1 AND product_id = $2")
            .bind(stock_item_id)
            .bind(input.product_id)
            .fetch_optional(conn)
            .await?;
    return match serial {
        None => Err(ApiError::bad_request_with_code(
            format!("line {}: that batch or serial does not belong to {}.", line_number, product.name),
            codes::STOCK_ITEM_REQUIRED,
        )),
        Some(Some(_)) if input.quantity != Decimal::ONE => Err(ApiError::bad_request(format!(
            "line {}: a serial-numbered item is sold one at a time",
            line_number
        ))),
        Some(_) => Ok(()),
    };
}
