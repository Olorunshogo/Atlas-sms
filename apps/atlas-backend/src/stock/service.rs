use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};

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
