use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};

use super::models::{AdjustStockRequest, LocationQuantity, MovementRecord, ProductInventory, StockLevel, StockMovement, TransferStockRequest};

/**
 * On-hand is never a stored column - it's `sum(quantity)` over the ledger for a store, grouped by product.
 * See `stock_movements`' migration comment for why: a mutable quantity column invites silent drift from the ledger that's supposed to be the source of truth.
 */
pub async fn on_hand_for_store(
    conn: &mut PgConnection,
    store_id: Uuid,
) -> Result<Vec<StockLevel>, ApiError> {
    return sqlx::query_as::<_, StockLevel>(
        "SELECT product_id, SUM(quantity) AS on_hand
         FROM atlas.stock_movements
         WHERE store_id = $1
         GROUP BY product_id
         ORDER BY product_id",
    )
    .bind(store_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * A count correction or write-off. Never blocked by the negative-stock check - a physical count is the authority the ledger is being corrected to, not a request that might overdraw it.
 */
pub async fn adjust_stock(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &AdjustStockRequest,
) -> Result<StockMovement, ApiError> {
    let lines = [PostingLine {
        product_id: payload.product_id,
        stock_item_id: payload.stock_item_id,
        unit_id: None,
        quantity: payload.quantity,
    }];
    let mut movements = inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: payload.store_id,
            movement_type: MovementType::Adjustment,
            reason: &payload.reason,
            reference_id: None,
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Allow,
            source: "adjustment",
            lines: &lines,
        },
    )
    .await?;
    return Ok(movements.remove(0));
}

/**
 * Two movements in one transaction: `transfer_out` at the source store (negative), `transfer_in` at the destination (positive).
 * Both carry the same `reference_id` so they're traceable as one transfer, not two unrelated adjustments.
 */
pub async fn transfer_stock(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &TransferStockRequest,
) -> Result<(StockMovement, StockMovement), ApiError> {
    if payload.quantity <= Decimal::ZERO {
        return Err(ApiError::bad_request("quantity must be positive"));
    }
    if payload.from_store_id == payload.to_store_id {
        return Err(ApiError::bad_request(
            "from_store_id and to_store_id must differ",
        ));
    }

    ensure_tradable(conn, tenant_id, &[payload.from_store_id, payload.to_store_id], payload.product_id).await?;

    let reference_id = Uuid::now_v7();
    let lines = [PostingLine {
        product_id: payload.product_id,
        stock_item_id: payload.stock_item_id,
        unit_id: None,
        quantity: payload.quantity,
    }];

    // The sending store can't give away stock it doesn't have.
    let mut out_movements = inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: payload.from_store_id,
            movement_type: MovementType::TransferOut,
            reason: &payload.reason,
            reference_id: Some(reference_id),
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Reject,
            source: "transfer",
            lines: &lines,
        },
    )
    .await?;

    let mut in_movements = inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: payload.to_store_id,
            movement_type: MovementType::TransferIn,
            reason: &payload.reason,
            reference_id: Some(reference_id),
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Allow,
            source: "transfer",
            lines: &lines,
        },
    )
    .await?;

    return Ok((out_movements.remove(0), in_movements.remove(0)));
}

/**
 * Stores and the product must exist in this tenant and be active.
 * Adjustments skip the active check on purpose, so a closed store's or discontinued product's leftover stock can still be written off.
 */
async fn ensure_tradable(conn: &mut PgConnection, tenant_id: Uuid, store_ids: &[Uuid], product_id: Uuid) -> Result<(), ApiError> {
    for store_id in store_ids {
        let is_active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM atlas.stores WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(&mut *conn)
            .await?;
        match is_active {
            None => return Err(ApiError::not_found("Store not found.")),
            Some(false) => return Err(ApiError::conflict_with_code("This store is not active.", codes::STORE_INACTIVE)),
            Some(true) => {}
        }
    }
    let is_active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM atlas.products WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL")
        .bind(product_id)
        .bind(tenant_id)
        .fetch_optional(&mut *conn)
        .await?;
    return match is_active {
        None => Err(ApiError::not_found("Product not found.")),
        Some(false) => Err(ApiError::conflict_with_code("This product is not active.", codes::PRODUCT_INACTIVE)),
        Some(true) => Ok(()),
    };
}

#[derive(sqlx::FromRow)]
struct LocationRow {
    product_id: Uuid,
    product_name: String,
    store_id: Uuid,
    store_name: String,
    quantity: Decimal,
}

/**
 * Products with their on-hand at each store in scope.
 * `scope` is `None` for every store in the tenant, otherwise the stores the caller may view; `total_quantity` is summed here from the per-store rows.
 * With `low_stock`, only locations at or below their listing's threshold are returned (and so only they count toward the total).
 */
pub async fn inventory_by_product(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    scope: Option<Vec<Uuid>>,
    store_id: Option<Uuid>,
    product_id: Option<Uuid>,
    low_stock: bool,
) -> Result<Vec<ProductInventory>, ApiError> {
    let rows = sqlx::query_as::<_, LocationRow>(
        "SELECT m.product_id, p.name AS product_name, m.store_id, s.name AS store_name, SUM(m.quantity) AS quantity
         FROM atlas.stock_movements m
         JOIN atlas.products p ON p.id = m.product_id AND p.deleted_at IS NULL
         JOIN atlas.stores s ON s.id = m.store_id AND s.deleted_at IS NULL
         LEFT JOIN atlas.product_store_listings l ON l.product_id = m.product_id AND l.store_id = m.store_id
         WHERE m.tenant_id = $1
           AND ($2::uuid[] IS NULL OR m.store_id = ANY($2))
           AND ($3::uuid IS NULL OR m.store_id = $3)
           AND ($4::uuid IS NULL OR m.product_id = $4)
         GROUP BY m.product_id, p.name, m.store_id, s.name, s.created_at, l.low_stock_threshold
         HAVING (NOT $5 OR SUM(m.quantity) <= COALESCE(l.low_stock_threshold, 0))
         ORDER BY p.name, m.product_id, s.created_at, m.store_id",
    )
    .bind(tenant_id)
    .bind(scope)
    .bind(store_id)
    .bind(product_id)
    .bind(low_stock)
    .fetch_all(conn)
    .await?;

    let mut products: Vec<ProductInventory> = Vec::new();
    for row in rows {
        let location = LocationQuantity {
            store_id: row.store_id,
            store_name: row.store_name,
            quantity: row.quantity,
        };
        match products.last_mut() {
            Some(last) if last.product_id == row.product_id => {
                last.total_quantity += location.quantity;
                last.locations.push(location);
            }
            _ => products.push(ProductInventory {
                product_id: row.product_id,
                product_name: row.product_name,
                total_quantity: location.quantity,
                locations: vec![location],
            }),
        }
    }
    return Ok(products);
}

/// One product across the stores in scope; an unknown product is a 404, a known one nobody has stocked is an empty list with a total of zero.
pub async fn product_inventory(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    scope: Option<Vec<Uuid>>,
    product_id: Uuid,
) -> Result<ProductInventory, ApiError> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM atlas.products WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL")
        .bind(product_id)
        .bind(tenant_id)
        .fetch_optional(&mut *conn)
        .await?;
    let name = name.ok_or_else(|| ApiError::not_found("Product not found."))?;
    let found = inventory_by_product(conn, tenant_id, scope, None, Some(product_id), false).await?;
    return Ok(found.into_iter().next().unwrap_or_else(|| ProductInventory {
        product_id,
        product_name: name,
        total_quantity: Decimal::ZERO,
        locations: Vec::new(),
    }));
}

/// The audit trail, newest first, limited to the stores in scope.
pub async fn list_movements(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    scope: Option<Vec<Uuid>>,
    store_id: Option<Uuid>,
    product_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<MovementRecord>, ApiError> {
    return sqlx::query_as::<_, MovementRecord>(
        "SELECT id, store_id, product_id, stock_item_id, movement_type::text AS movement_type, quantity, reason,
                reference_id, created_by_user_id, created_at
         FROM atlas.stock_movements
         WHERE tenant_id = $1
           AND ($2::uuid[] IS NULL OR store_id = ANY($2))
           AND ($3::uuid IS NULL OR store_id = $3)
           AND ($4::uuid IS NULL OR product_id = $4)
         ORDER BY created_at DESC, id DESC
         LIMIT $5",
    )
    .bind(tenant_id)
    .bind(scope)
    .bind(store_id)
    .bind(product_id)
    .bind(limit)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}
