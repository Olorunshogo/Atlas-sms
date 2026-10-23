use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::stock::service::{self as stock_service, Actor, Entry};

use super::models::{
    CloseShiftRequest, CreateSaleRequest, OpenShiftRequest, Sale, SaleLine, Shift,
};

// === Shifts

pub async fn open_shift(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &OpenShiftRequest,
) -> Result<Shift, ApiError> {
    sqlx::query_as::<_, Shift>(
        "INSERT INTO atlas.shifts (id, tenant_id, store_id, opened_by_user_id, opening_cash_amount_minor)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, store_id, opened_by_user_id, opened_at, opening_cash_amount_minor,
                   closed_by_user_id, closed_at, closing_cash_amount_minor, status::text AS status",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(user_id)
    .bind(payload.opening_cash_amount_minor)
    .fetch_one(conn)
    .await
    .map_err(|err| match err {
        // shifts_one_open_per_store_idx - the store already has an open shift.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("This store already has an open shift.", "SHIFT_ALREADY_OPEN")
        }
        other => ApiError::from(other),
    })
}

pub async fn close_shift(
    conn: &mut PgConnection,
    shift_id: Uuid,
    user_id: Uuid,
    payload: &CloseShiftRequest,
) -> Result<Shift, ApiError> {
    sqlx::query_as::<_, Shift>(
        "UPDATE atlas.shifts
         SET status = 'closed', closed_by_user_id = $2, closed_at = now(), closing_cash_amount_minor = $3
         WHERE id = $1 AND status = 'open'
         RETURNING id, store_id, opened_by_user_id, opened_at, opening_cash_amount_minor,
                   closed_by_user_id, closed_at, closing_cash_amount_minor, status::text AS status",
    )
    .bind(shift_id)
    .bind(user_id)
    .bind(payload.closing_cash_amount_minor)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Shift not found or already closed."))
}

// === Sales

/// A request line after the server has resolved everything it must not take on trust.
struct ResolvedLine {
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
    unit_id: Uuid,
    quantity: Decimal,
    base_quantity: Decimal,
    unit_price: i64,
    line_total: i64,
}

/**
 * Idempotent by `payload.id` (client-generated) - both the direct
 * `POST /api/v1/sales` path and the offline `POST /api/v1/sync/sales` path
 * call this same function. A sale synced twice (a flaky connection
 * retried) finds its own id already present and returns it unchanged
 * rather than double-charging stock. Stock is only ever adjusted the
 * first time a given sale id is accepted.
 *
 * Runs inside the caller's transaction, so any error (most commonly
 * `INSUFFICIENT_STOCK` on one line of several) drops the transaction and
 * rolls back the sale, its lines and every stock change together.
 *
 * Prices come from `price_rules` and stock is checked and decremented
 * atomically per line; nothing about money or availability is taken from
 * the request. Discount and tax are the cashier's input (there are no tax
 * rules in the schema yet) and are only bounds-checked.
 */
pub async fn create_sale(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    cashier_user_id: Uuid,
    payload: &CreateSaleRequest,
) -> Result<Sale, ApiError> {
    if let Some(existing) = fetch_sale(conn, payload.id).await? {
        return Ok(existing);
    }

    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a sale must have at least one line"));
    }
    if payload.discount_amount_minor < 0 || payload.tax_amount_minor < 0 {
        return Err(ApiError::bad_request("discount and tax cannot be negative"));
    }

    stock_service::ensure_store(conn, payload.store_id, true).await?;
    ensure_shift_at_store(conn, payload.shift_id, payload.store_id).await?;
    let customer_group_id = customer_group(conn, payload.customer_id).await?;

    let mut lines = Vec::with_capacity(payload.lines.len());
    for line in &payload.lines {
        lines.push(resolve_line(conn, payload.store_id, customer_group_id, line).await?);
    }
    // Same lock order for every sale, so two multi-line sales cannot deadlock.
    lines.sort_by_key(|line| (line.product_id, line.stock_item_id));

    let subtotal = lines
        .iter()
        .try_fold(0_i64, |sum, line| sum.checked_add(line.line_total))
        .ok_or_else(|| ApiError::bad_request("sale total out of range"))?;
    if payload.discount_amount_minor > subtotal {
        return Err(ApiError::bad_request("discount cannot exceed the subtotal"));
    }
    let total = subtotal - payload.discount_amount_minor + payload.tax_amount_minor;

    let inserted = sqlx::query_as::<_, Sale>(
        "INSERT INTO atlas.sales
            (id, tenant_id, store_id, shift_id, customer_id, cashier_user_id, device_id,
             subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, synced_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())
         ON CONFLICT (id) DO NOTHING
         RETURNING id, store_id, shift_id, customer_id, cashier_user_id, device_id, status::text AS status,
                   subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at, voided_at",
    )
    .bind(payload.id)
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(payload.shift_id)
    .bind(payload.customer_id)
    .bind(cashier_user_id)
    .bind(&payload.device_id)
    .bind(subtotal)
    .bind(payload.discount_amount_minor)
    .bind(payload.tax_amount_minor)
    .bind(total)
    .fetch_optional(&mut *conn)
    .await?;

    /*
     * A concurrent request with the same id committed between our first
     * lookup and this insert: it won, so return its sale untouched.
     */
    let Some(sale) = inserted else {
        return fetch_sale(conn, payload.id)
            .await?
            .ok_or_else(|| ApiError::Internal("sale vanished after conflict".to_string()));
    };

    let actor = Actor {
        tenant_id,
        user_id: cashier_user_id,
    };
    for line in &lines {
        sqlx::query(
            "INSERT INTO atlas.sale_lines
                (id, tenant_id, sale_id, product_id, stock_item_id, unit_id, quantity, base_quantity, unit_price_amount_minor, line_total_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(sale.id)
        .bind(line.product_id)
        .bind(line.stock_item_id)
        .bind(line.unit_id)
        .bind(line.quantity)
        .bind(line.base_quantity)
        .bind(line.unit_price)
        .bind(line.line_total)
        .execute(&mut *conn)
        .await?;

        stock_service::debit(
            conn,
            actor,
            &Entry {
                store_id: payload.store_id,
                product_id: line.product_id,
                stock_item_id: line.stock_item_id,
                movement_type: "sale",
                quantity: line.base_quantity,
                reason: "Sale",
                reference_id: Some(sale.id),
            },
        )
        .await?;
    }

    Ok(sale)
}

/**
 * Only checks the shift exists and belongs to this store - not that it is
 * still open, because a sale rung up offline is synced after its shift may
 * have closed.
 */
async fn ensure_shift_at_store(
    conn: &mut PgConnection,
    shift_id: Uuid,
    store_id: Uuid,
) -> Result<(), ApiError> {
    let found: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM atlas.shifts WHERE id = $1 AND store_id = $2")
            .bind(shift_id)
            .bind(store_id)
            .fetch_optional(conn)
            .await?;
    found
        .map(|_| ())
        .ok_or_else(|| ApiError::NotFound("Shift not found at this store.".to_string()))
}

async fn customer_group(
    conn: &mut PgConnection,
    customer_id: Option<Uuid>,
) -> Result<Option<Uuid>, ApiError> {
    let Some(customer_id) = customer_id else {
        return Ok(None);
    };
    let row: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT customer_group_id FROM atlas.customers WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(customer_id)
    .fetch_optional(conn)
    .await?;
    row.ok_or_else(|| ApiError::NotFound("Customer not found.".to_string()))
}

/**
 * Validates the line, converts its quantity to the product's base unit
 * (stock is held in base units) and prices it. Price rules are per base
 * unit: the line total is `price x base quantity`, and the stored
 * per-line unit price is that price scaled to the sale unit.
 */
async fn resolve_line(
    conn: &mut PgConnection,
    store_id: Uuid,
    customer_group_id: Option<Uuid>,
    line: &super::models::SaleLineInput,
) -> Result<ResolvedLine, ApiError> {
    stock_service::require_positive(line.quantity)?;
    stock_service::ensure_product_item(conn, line.product_id, line.stock_item_id, true).await?;

    let factor: Option<Decimal> = sqlx::query_scalar(
        "SELECT CASE WHEN p.base_unit_id = $2 THEN 1::numeric
                     ELSE (SELECT factor_to_base FROM atlas.product_sale_units
                           WHERE product_id = p.id AND unit_id = $2)
                END
         FROM atlas.products p WHERE p.id = $1",
    )
    .bind(line.product_id)
    .bind(line.unit_id)
    .fetch_one(&mut *conn)
    .await?;
    let factor = factor.ok_or_else(|| {
        ApiError::bad_request_with_code("unit is not a sale unit of this product", "INVALID_UNIT")
    })?;

    let out_of_range = || ApiError::bad_request("line total out of range");
    let base_quantity = line.quantity.checked_mul(factor).ok_or_else(out_of_range)?;
    let price = resolve_unit_price(
        conn,
        line.product_id,
        store_id,
        customer_group_id,
        base_quantity,
    )
    .await?;

    let line_total = Decimal::from(price)
        .checked_mul(base_quantity)
        .map(|total| total.round())
        .and_then(|total| total.to_i64())
        .ok_or_else(out_of_range)?;
    let unit_price = Decimal::from(price)
        .checked_mul(factor)
        .map(|unit| unit.round())
        .and_then(|unit| unit.to_i64())
        .ok_or_else(out_of_range)?;

    Ok(ResolvedLine {
        product_id: line.product_id,
        stock_item_id: line.stock_item_id,
        unit_id: line.unit_id,
        quantity: line.quantity,
        base_quantity,
        unit_price,
        line_total,
    })
}

/**
 * The most specific active rule wins: a store-specific rule beats an
 * all-stores one, a customer-group rule beats an all-customers one, then
 * the highest `min_quantity` tier reached, then `priority`. No matching
 * rule is an error - a product with no price is not sold for free.
 */
async fn resolve_unit_price(
    conn: &mut PgConnection,
    product_id: Uuid,
    store_id: Uuid,
    customer_group_id: Option<Uuid>,
    base_quantity: Decimal,
) -> Result<i64, ApiError> {
    let price: Option<i64> = sqlx::query_scalar(
        "SELECT unit_price_amount_minor
         FROM atlas.price_rules
         WHERE product_id = $1
           AND (store_id IS NULL OR store_id = $2)
           AND (customer_group_id IS NULL OR customer_group_id = $3)
           AND min_quantity <= $4
           AND (starts_at IS NULL OR starts_at <= now())
           AND (ends_at IS NULL OR ends_at > now())
         ORDER BY (store_id IS NOT NULL) DESC, (customer_group_id IS NOT NULL) DESC,
                  min_quantity DESC, priority DESC, created_at DESC
         LIMIT 1",
    )
    .bind(product_id)
    .bind(store_id)
    .bind(customer_group_id)
    .bind(base_quantity)
    .fetch_optional(conn)
    .await?;
    price.ok_or_else(|| {
        ApiError::bad_request_with_code("no price is set for this product", "NO_PRICE")
    })
}

pub(super) async fn fetch_sale(
    conn: &mut PgConnection,
    sale_id: Uuid,
) -> Result<Option<Sale>, ApiError> {
    sqlx::query_as::<_, Sale>(
        "SELECT id, store_id, shift_id, customer_id, cashier_user_id, device_id, status::text AS status,
                subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at, voided_at
         FROM atlas.sales WHERE id = $1",
    )
    .bind(sale_id)
    .fetch_optional(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn list_sales_for_shift(
    conn: &mut PgConnection,
    shift_id: Uuid,
) -> Result<Vec<Sale>, ApiError> {
    sqlx::query_as::<_, Sale>(
        "SELECT id, store_id, shift_id, customer_id, cashier_user_id, device_id, status::text AS status,
                subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at, voided_at
         FROM atlas.sales WHERE shift_id = $1 ORDER BY created_at",
    )
    .bind(shift_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn list_sale_lines(
    conn: &mut PgConnection,
    sale_id: Uuid,
) -> Result<Vec<SaleLine>, ApiError> {
    sqlx::query_as::<_, SaleLine>(
        "SELECT id, sale_id, product_id, stock_item_id, unit_id, quantity, base_quantity,
                unit_price_amount_minor, line_total_amount_minor
         FROM atlas.sale_lines WHERE sale_id = $1 ORDER BY id",
    )
    .bind(sale_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}
