/*!
 * FR-14 batch rules shared by every stock writer: which products are batch-tracked, finding or creating a batch, whether a batch may be sold, and first-expiry-first-out allocation.
 * "Expired" means the expiry date is before today in the store's own timezone; a batch is still sellable on its expiry day.
 */
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};

pub async fn tracks_batches(conn: &mut PgConnection, product_id: Uuid) -> Result<bool, ApiError> {
    let tracked: Option<bool> = sqlx::query_scalar(
        "SELECT pt.identity_mode IN ('by_batch', 'by_both')
         FROM atlas.products p JOIN atlas.product_types pt ON pt.id = p.product_type_id
         WHERE p.id = $1",
    )
    .bind(product_id)
    .fetch_optional(conn)
    .await?;
    return Ok(tracked.unwrap_or(false));
}

/// Batch-tracked products must name a batch on every manual posting, and a named batch must be one of the product's own.
pub async fn check_batch_given(conn: &mut PgConnection, product_id: Uuid, stock_item_id: Option<Uuid>) -> Result<(), ApiError> {
    match stock_item_id {
        Some(stock_item_id) => {
            let owner: Option<Uuid> = sqlx::query_scalar("SELECT product_id FROM atlas.stock_items WHERE id = $1")
                .bind(stock_item_id)
                .fetch_optional(conn)
                .await?;
            if owner != Some(product_id) {
                return Err(ApiError::not_found("Batch not found for this product."));
            }
        }
        None if tracks_batches(conn, product_id).await? => {
            return Err(ApiError::bad_request_with_code("This product is batch-tracked; name the batch.", codes::BATCH_REQUIRED));
        }
        None => {}
    }
    return Ok(());
}

/// The batch a returned item most likely came from: the earliest-expiring one this sale took the product from.
pub async fn sold_batch(conn: &mut PgConnection, sale_id: Uuid, product_id: Uuid) -> Result<Option<Uuid>, ApiError> {
    return sqlx::query_scalar(
        "SELECT m.stock_item_id
         FROM atlas.stock_movements m JOIN atlas.stock_items si ON si.id = m.stock_item_id
         WHERE m.reference_id = $1 AND m.movement_type = 'sale' AND m.product_id = $2
         ORDER BY si.expiry_date NULLS LAST, si.id
         LIMIT 1",
    )
    .bind(sale_id)
    .bind(product_id)
    .fetch_optional(conn)
    .await
    .map_err(ApiError::from);
}

/// The same batch number received twice is the same batch; a conflicting expiry date is refused rather than overwritten.
pub async fn find_or_create_batch(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    product_id: Uuid,
    batch_number: &str,
    expiry_date: Option<NaiveDate>,
) -> Result<Uuid, ApiError> {
    let existing: Option<(Uuid, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, expiry_date FROM atlas.stock_items
         WHERE product_id = $1 AND batch_number = $2 AND serial_number IS NULL",
    )
    .bind(product_id)
    .bind(batch_number)
    .fetch_optional(&mut *conn)
    .await?;
    match existing {
        Some((_, Some(known))) if expiry_date.is_some_and(|given| given != known) => {
            return Err(ApiError::bad_request_with_code(
                format!("Batch {} already exists with expiry {}.", batch_number, known),
                "VALIDATION_FAILED",
            ));
        }
        Some((id, None)) if expiry_date.is_some() => {
            sqlx::query("UPDATE atlas.stock_items SET expiry_date = $2 WHERE id = $1")
                .bind(id)
                .bind(expiry_date)
                .execute(&mut *conn)
                .await?;
            return Ok(id);
        }
        Some((id, _)) => return Ok(id),
        None => {}
    }
    return sqlx::query_scalar(
        "INSERT INTO atlas.stock_items (id, tenant_id, product_id, batch_number, expiry_date)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(batch_number)
    .bind(expiry_date)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn allow_expired_sale(conn: &mut PgConnection, store_id: Uuid) -> Result<bool, ApiError> {
    let allowed: Option<bool> = sqlx::query_scalar("SELECT allow_expired_sale FROM atlas.stores WHERE id = $1")
        .bind(store_id)
        .fetch_optional(conn)
        .await?;
    return Ok(allowed.unwrap_or(false));
}

/// Refuses a named batch that has expired at this store, unless the store allows selling expired stock.
pub async fn assert_sellable(conn: &mut PgConnection, store_id: Uuid, stock_item_id: Uuid, allow_expired: bool) -> Result<(), ApiError> {
    if allow_expired {
        return Ok(());
    }
    let expired: Option<(bool, String)> = sqlx::query_as(
        "SELECT si.expiry_date < (now() AT TIME ZONE s.timezone)::date, COALESCE(si.batch_number, si.serial_number)
         FROM atlas.stock_items si, atlas.stores s
         WHERE si.id = $1 AND s.id = $2 AND si.expiry_date IS NOT NULL",
    )
    .bind(stock_item_id)
    .bind(store_id)
    .fetch_optional(conn)
    .await?;
    if let Some((true, label)) = expired {
        return Err(ApiError::conflict_with_code(format!("Batch {} has expired and can't be sold.", label), codes::BATCH_EXPIRED));
    }
    return Ok(());
}

/**
 * Splits `quantity` (base units) across the product's batches at the store, earliest expiry first, then batches with no expiry, then any stock held without a batch (`None`).
 * Returns the allocations and the shortfall nothing covered. Call with the product's stock lock held.
 */
pub async fn allocate_fefo(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
    quantity: Decimal,
    include_expired: bool,
) -> Result<(Vec<(Option<Uuid>, Decimal)>, Decimal), ApiError> {
    let batches: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT si.id, SUM(m.quantity)
         FROM atlas.stock_movements m
         JOIN atlas.stock_items si ON si.id = m.stock_item_id
         JOIN atlas.stores s ON s.id = m.store_id
         WHERE m.store_id = $1 AND m.product_id = $2
           AND ($3 OR si.expiry_date IS NULL OR si.expiry_date >= (now() AT TIME ZONE s.timezone)::date)
         GROUP BY si.id, si.expiry_date, si.created_at
         HAVING SUM(m.quantity) > 0
         ORDER BY si.expiry_date NULLS LAST, si.created_at, si.id",
    )
    .bind(store_id)
    .bind(product_id)
    .bind(include_expired)
    .fetch_all(&mut *conn)
    .await?;
    let unbatched: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(quantity), 0) FROM atlas.stock_movements WHERE store_id = $1 AND product_id = $2 AND stock_item_id IS NULL",
    )
    .bind(store_id)
    .bind(product_id)
    .fetch_one(conn)
    .await?;

    let mut sources: Vec<(Option<Uuid>, Decimal)> = batches.into_iter().map(|(id, available)| (Some(id), available)).collect();
    if unbatched > Decimal::ZERO {
        sources.push((None, unbatched));
    }
    let mut remaining = quantity;
    let mut allocations = Vec::new();
    for (stock_item_id, available) in sources {
        if remaining <= Decimal::ZERO {
            break;
        }
        let take = remaining.min(available);
        allocations.push((stock_item_id, take));
        remaining -= take;
    }
    return Ok((allocations, remaining));
}
