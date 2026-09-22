use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

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

/**
 * Idempotent by `payload.id` (client-generated) - both the direct
 * `POST /api/v1/sales` path and the offline `POST /api/v1/sync/sales` path
 * call this same function. A sale synced twice (a flaky connection
 * retried) finds its own id already present and returns it unchanged
 * rather than double-charging stock. Stock is only ever adjusted the
 * first time a given sale id is accepted.
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

    let line_totals = payload
        .lines
        .iter()
        .map(line_total_minor)
        .collect::<Result<Vec<i64>, ApiError>>()?;
    let subtotal: i64 = line_totals.iter().sum();
    let total = subtotal - payload.discount_amount_minor + payload.tax_amount_minor;

    let sale = sqlx::query_as::<_, Sale>(
        "INSERT INTO atlas.sales
            (id, tenant_id, store_id, shift_id, customer_id, cashier_user_id, device_id,
             subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, synced_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())
         RETURNING id, store_id, shift_id, customer_id, cashier_user_id, device_id, status::text AS status,
                   subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at",
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
    .fetch_one(&mut *conn)
    .await?;

    for (line, line_total) in payload.lines.iter().zip(line_totals) {
        sqlx::query(
            "INSERT INTO atlas.sale_lines
                (id, tenant_id, sale_id, product_id, stock_item_id, unit_id, quantity, unit_price_amount_minor, line_total_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(sale.id)
        .bind(line.product_id)
        .bind(line.stock_item_id)
        .bind(line.unit_id)
        .bind(line.quantity)
        .bind(line.unit_price_amount_minor)
        .bind(line_total)
        .execute(&mut *conn)
        .await?;

        sqlx::query(
            "INSERT INTO atlas.stock_movements
                (id, tenant_id, store_id, product_id, stock_item_id, movement_type, quantity, reason, reference_id, created_by_user_id)
             VALUES ($1, $2, $3, $4, $5, 'sale', $6, 'Sale', $7, $8)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(payload.store_id)
        .bind(line.product_id)
        .bind(line.stock_item_id)
        .bind(-line.quantity)
        .bind(sale.id)
        .bind(cashier_user_id)
        .execute(&mut *conn)
        .await?;
    }

    Ok(sale)
}

/**
 * Proper decimal money arithmetic, not integer truncation of `quantity` -
 * a 0.5kg line at a unit price still needs to bill half, not zero. Rounds
 * to the nearest minor unit (kobo/cent) since the ledger's smallest
 * representable amount is one.
 */
fn line_total_minor(line: &super::models::SaleLineInput) -> Result<i64, ApiError> {
    let total = (Decimal::from(line.unit_price_amount_minor) * line.quantity).round();
    total
        .to_i64()
        .ok_or_else(|| ApiError::bad_request("line total out of range"))
}

async fn fetch_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<Option<Sale>, ApiError> {
    sqlx::query_as::<_, Sale>(
        "SELECT id, store_id, shift_id, customer_id, cashier_user_id, device_id, status::text AS status,
                subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at
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
                subtotal_amount_minor, discount_amount_minor, tax_amount_minor, total_amount_minor, created_at
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
        "SELECT id, sale_id, product_id, stock_item_id, unit_id, quantity,
                unit_price_amount_minor, line_total_amount_minor
         FROM atlas.sale_lines WHERE sale_id = $1 ORDER BY id",
    )
    .bind(sale_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}
