use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};

use super::models::{
    AdjustStockRequest, LocationQuantity, MovementRecord, ProductInventory, RequestTransferRequest,
    StockLevel, StockMovement, StockTransfer,
};

const TRANSFER_COLUMNS: &str = "id, from_store_id, to_store_id, product_id, stock_item_id, quantity, reason, status,
     requested_by_user_id, requested_at, approved_by_user_id, approved_at,
     dispatched_at, received_at, cancelled_by_user_id, cancelled_at";

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

/// Same ledger-sum approach as `on_hand_for_store`, scoped to one product - what a transfer request needs to check.
async fn on_hand_for_product(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<Decimal, ApiError> {
    sqlx::query_scalar::<_, Option<Decimal>>(
        "SELECT SUM(quantity) FROM atlas.stock_movements WHERE store_id = $1 AND product_id = $2",
    )
    .bind(store_id)
    .bind(product_id)
    .fetch_one(conn)
    .await
    .map(|sum| sum.unwrap_or(Decimal::ZERO))
    .map_err(ApiError::from)
}

/// A direct correction. Can't take on-hand below zero unless the store's `allow_negative_stock` policy permits it (FR-12); a physical count corrects through `counts` instead.
pub async fn adjust_stock(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &AdjustStockRequest,
) -> Result<StockMovement, ApiError> {
    inventory::batches::check_batch_given(conn, payload.product_id, payload.stock_item_id).await?;
    let allow_negative: Option<bool> = sqlx::query_scalar("SELECT allow_negative_stock FROM atlas.stores WHERE id = $1 AND deleted_at IS NULL")
        .bind(payload.store_id)
        .fetch_optional(&mut *conn)
        .await?;
    let policy = match allow_negative {
        Some(true) => NegativeStockPolicy::Allow,
        _ => NegativeStockPolicy::Reject,
    };
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
            policy,
            source: "adjustment",
            lines: &lines,
        },
    )
    .await?;
    return Ok(movements.remove(0));
}

// === Inter-store transfers (FR-22): requested -> approved -> dispatched -> received, reject/cancel as exits.

pub async fn request_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &RequestTransferRequest,
) -> Result<StockTransfer, ApiError> {
    if payload.quantity <= Decimal::ZERO {
        return Err(ApiError::bad_request("quantity must be positive"));
    }
    if payload.from_store_id == payload.to_store_id {
        return Err(ApiError::bad_request(
            "from_store_id and to_store_id must differ",
        ));
    }

    ensure_tradable(conn, tenant_id, &[payload.from_store_id, payload.to_store_id], payload.product_id).await?;

    // Not a hold - just an early, honest rejection. The authoritative check
    // that actually prevents overselling happens at dispatch.
    let available = on_hand_for_product(conn, payload.from_store_id, payload.product_id).await?;
    if available < payload.quantity {
        return Err(ApiError::conflict_with_code(
            "Source store does not have enough stock for this transfer.",
            codes::PRODUCT_OUT_OF_STOCK,
        ));
    }

    return sqlx::query_as::<_, StockTransfer>(&format!(
        "INSERT INTO atlas.stock_transfers
            (id, tenant_id, from_store_id, to_store_id, product_id, stock_item_id, quantity, reason, requested_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.from_store_id)
    .bind(payload.to_store_id)
    .bind(payload.product_id)
    .bind(payload.stock_item_id)
    .bind(payload.quantity)
    .bind(&payload.reason)
    .bind(user_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn get_transfer(conn: &mut PgConnection, transfer_id: Uuid) -> Result<StockTransfer, ApiError> {
    return sqlx::query_as::<_, StockTransfer>(&format!(
        "SELECT {TRANSFER_COLUMNS} FROM atlas.stock_transfers WHERE id = $1"
    ))
    .bind(transfer_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Transfer not found."));
}

pub async fn list_transfers(conn: &mut PgConnection, store_id: Uuid) -> Result<Vec<StockTransfer>, ApiError> {
    return sqlx::query_as::<_, StockTransfer>(&format!(
        "SELECT {TRANSFER_COLUMNS} FROM atlas.stock_transfers
         WHERE from_store_id = $1 OR to_store_id = $1
         ORDER BY requested_at DESC"
    ))
    .bind(store_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn approve_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    transfer_id: Uuid,
) -> Result<StockTransfer, ApiError> {
    return sqlx::query_as::<_, StockTransfer>(&format!(
        "UPDATE atlas.stock_transfers
         SET status = 'approved', approved_by_user_id = $1, approved_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status = 'requested'
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(user_id)
    .bind(transfer_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a requested transfer can be approved.",
        codes::TRANSFER_NOT_REQUESTED,
    ));
}

pub async fn reject_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    transfer_id: Uuid,
) -> Result<StockTransfer, ApiError> {
    return sqlx::query_as::<_, StockTransfer>(&format!(
        "UPDATE atlas.stock_transfers
         SET status = 'rejected'
         WHERE id = $1 AND tenant_id = $2 AND status = 'requested'
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(transfer_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a requested transfer can be rejected.",
        codes::TRANSFER_NOT_REQUESTED,
    ));
}

pub async fn cancel_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    transfer_id: Uuid,
) -> Result<StockTransfer, ApiError> {
    return sqlx::query_as::<_, StockTransfer>(&format!(
        "UPDATE atlas.stock_transfers
         SET status = 'cancelled', cancelled_by_user_id = $1, cancelled_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status IN ('requested', 'approved')
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(user_id)
    .bind(transfer_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a requested or approved transfer can be cancelled (dispatched stock has already left).",
        codes::TRANSFER_NOT_CANCELLABLE,
    ));
}

// Claims the status transition first, so two concurrent dispatches can't both post the movement.
pub async fn dispatch_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    transfer_id: Uuid,
) -> Result<StockTransfer, ApiError> {
    let claimed = sqlx::query_as::<_, StockTransfer>(&format!(
        "UPDATE atlas.stock_transfers
         SET status = 'dispatched', dispatched_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'approved'
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(transfer_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only an approved transfer can be dispatched.",
        codes::TRANSFER_NOT_APPROVED,
    ))?;

    let lines = dispatch_lines(conn, &claimed).await?;
    inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: claimed.from_store_id,
            movement_type: MovementType::TransferOut,
            reason: &claimed.reason,
            reference_id: Some(claimed.id),
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Reject,
            source: "transfer",
            lines: &lines,
        },
    )
    .await?;

    return Ok(claimed);
}

/**
 * A named batch moves as named. A batch-tracked product with none named is split across the source's batches
 * first-expiry-first-out, expired batches left behind - the same rule a sale follows (FR-14).
 */
async fn dispatch_lines(conn: &mut PgConnection, transfer: &StockTransfer) -> Result<Vec<PostingLine>, ApiError> {
    let whole = PostingLine { product_id: transfer.product_id, stock_item_id: transfer.stock_item_id, unit_id: None, quantity: transfer.quantity };
    if transfer.stock_item_id.is_some() || !inventory::batches::tracks_batches(conn, transfer.product_id).await? {
        return Ok(vec![whole]);
    }
    inventory::service::lock_products(conn, transfer.from_store_id, &[transfer.product_id]).await?;
    let (allocations, remainder) = inventory::batches::allocate_fefo(conn, transfer.from_store_id, transfer.product_id, transfer.quantity, false).await?;
    if remainder > Decimal::ZERO {
        return Err(ApiError::conflict_with_code(
            format!(
                "Not enough unexpired stock to send: requested {}, available {}.",
                transfer.quantity.normalize(),
                (transfer.quantity - remainder).normalize()
            ),
            codes::PRODUCT_OUT_OF_STOCK,
        ));
    }
    return Ok(allocations
        .into_iter()
        .map(|(stock_item_id, quantity)| PostingLine { product_id: transfer.product_id, stock_item_id, unit_id: None, quantity })
        .collect());
}

/// Mirrors `dispatch_transfer`: claim the transition first, then post what it sent.
pub async fn receive_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    transfer_id: Uuid,
) -> Result<StockTransfer, ApiError> {
    let claimed = sqlx::query_as::<_, StockTransfer>(&format!(
        "UPDATE atlas.stock_transfers
         SET status = 'received', received_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'dispatched'
         RETURNING {TRANSFER_COLUMNS}"
    ))
    .bind(transfer_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a dispatched transfer can be received.",
        codes::TRANSFER_NOT_DISPATCHED,
    ))?;

    // The same batches arrive that left, whatever dispatch picked
    let sent: Vec<(Option<Uuid>, Decimal)> = sqlx::query_as(
        "SELECT stock_item_id, -quantity FROM atlas.stock_movements
         WHERE reference_id = $1 AND movement_type = 'transfer_out' AND store_id = $2 ORDER BY id",
    )
    .bind(claimed.id)
    .bind(claimed.from_store_id)
    .fetch_all(&mut *conn)
    .await?;
    let lines: Vec<PostingLine> = sent
        .into_iter()
        .map(|(stock_item_id, quantity)| PostingLine { product_id: claimed.product_id, stock_item_id, unit_id: None, quantity })
        .collect();
    inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id: claimed.to_store_id,
            movement_type: MovementType::TransferIn,
            reason: &claimed.reason,
            reference_id: Some(claimed.id),
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Allow,
            source: "transfer",
            lines: &lines,
        },
    )
    .await?;

    return Ok(claimed);
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
    category_id: Option<Uuid>,
    search: Option<&str>,
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
           AND ($6::uuid IS NULL OR p.category_id = $6)
           AND ($7::text IS NULL OR p.name ILIKE '%' || $7 || '%' OR p.sku ILIKE '%' || $7 || '%')
         GROUP BY m.product_id, p.name, m.store_id, s.name, s.created_at, l.low_stock_threshold
         HAVING (NOT $5 OR SUM(m.quantity) <= COALESCE(l.low_stock_threshold, 0))
         ORDER BY p.name, m.product_id, s.created_at, m.store_id",
    )
    .bind(tenant_id)
    .bind(scope)
    .bind(store_id)
    .bind(product_id)
    .bind(low_stock)
    .bind(category_id)
    .bind(search)
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
    let found = inventory_by_product(conn, tenant_id, scope, None, Some(product_id), None, None, false).await?;
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
