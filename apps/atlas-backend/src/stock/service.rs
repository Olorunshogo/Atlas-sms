use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{AdjustStockRequest, StockLevel, StockMovement, TransferStockRequest};

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

struct NewMovement<'a> {
    tenant_id: Uuid,
    store_id: Uuid,
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
    movement_type: &'a str,
    quantity: Decimal,
    reason: &'a str,
    reference_id: Option<Uuid>,
    created_by_user_id: Uuid,
}

async fn insert_movement(
    conn: &mut PgConnection,
    movement: NewMovement<'_>,
) -> Result<StockMovement, ApiError> {
    return sqlx::query_as::<_, StockMovement>(
        "INSERT INTO atlas.stock_movements
            (id, tenant_id, store_id, product_id, stock_item_id, movement_type, quantity, reason, reference_id, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6::atlas.stock_movement_type, $7, $8, $9, $10)
         RETURNING id, store_id, product_id, stock_item_id, movement_type::text AS movement_type, quantity, reason",
    )
    .bind(Uuid::now_v7())
    .bind(movement.tenant_id)
    .bind(movement.store_id)
    .bind(movement.product_id)
    .bind(movement.stock_item_id)
    .bind(movement.movement_type)
    .bind(movement.quantity)
    .bind(movement.reason)
    .bind(movement.reference_id)
    .bind(movement.created_by_user_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn adjust_stock(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &AdjustStockRequest,
) -> Result<StockMovement, ApiError> {
    return insert_movement(
        conn,
        NewMovement {
            tenant_id,
            store_id: payload.store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "adjustment",
            quantity: payload.quantity,
            reason: &payload.reason,
            reference_id: None,
            created_by_user_id: user_id,
        },
    )
    .await;
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

    let reference_id = Uuid::now_v7();

    let out_movement = insert_movement(
        conn,
        NewMovement {
            tenant_id,
            store_id: payload.from_store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "transfer_out",
            quantity: -payload.quantity,
            reason: &payload.reason,
            reference_id: Some(reference_id),
            created_by_user_id: user_id,
        },
    )
    .await?;

    let in_movement = insert_movement(
        conn,
        NewMovement {
            tenant_id,
            store_id: payload.to_store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "transfer_in",
            quantity: payload.quantity,
            reason: &payload.reason,
            reference_id: Some(reference_id),
            created_by_user_id: user_id,
        },
    )
    .await?;

    return Ok((out_movement, in_movement));
}
