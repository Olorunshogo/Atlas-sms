use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{
    AdjustStockRequest, InitialStockRequest, LocationQuantity, ProductInventory, RestockRequest,
    StockLevel, StockMovement, TransferStockRequest,
};

// === Types

/** Who is making the change, carried into every ledger row. */
#[derive(Clone, Copy)]
pub struct Actor {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

/**
 * One ledger entry to write. `quantity` is always the positive magnitude;
 * `credit` and `debit` decide the sign stored in `stock_movements`.
 */
pub struct Entry<'a> {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub movement_type: &'a str,
    pub quantity: Decimal,
    pub reason: &'a str,
    pub reference_id: Option<Uuid>,
}

const MOVEMENT_COLUMNS: &str = "id, store_id, product_id, stock_item_id, movement_type::text AS movement_type, quantity, reason, reference_id, created_by_user_id, created_at";

// === Validation

pub fn require_positive(quantity: Decimal) -> Result<(), ApiError> {
    if quantity <= Decimal::ZERO {
        return Err(ApiError::bad_request_with_code(
            "quantity must be greater than zero",
            "INVALID_QUANTITY",
        ));
    }
    Ok(())
}

pub fn require_reason(reason: &str) -> Result<&str, ApiError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request_with_code(
            "a reason is required",
            "REASON_REQUIRED",
        ));
    }
    Ok(reason)
}

/**
 * Foreign keys do not go through RLS, so a tenant could otherwise name
 * another tenant's store id and have it accepted. Reading through the
 * scoped transaction turns a foreign id into "not found". With
 * `require_active`, a deactivated store is refused too (409); adjustments
 * pass `false` so a closed store's stock can still be written off.
 */
pub async fn ensure_store(
    conn: &mut PgConnection,
    store_id: Uuid,
    require_active: bool,
) -> Result<(), ApiError> {
    let is_active: Option<bool> = sqlx::query_scalar(
        "SELECT is_active FROM atlas.stores WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(store_id)
    .fetch_optional(conn)
    .await?;
    match is_active {
        None => Err(ApiError::NotFound("Store not found.".to_string())),
        Some(false) if require_active => Err(ApiError::conflict_with_code(
            "This store is not active.",
            "STORE_INACTIVE",
        )),
        Some(_) => Ok(()),
    }
}

/**
 * Checks the product exists (and is active, with `require_active`) and
 * that `stock_item_id` matches its identity mode: a by-product item must not carry a batch or serial, a tracked one
 * must name one that belongs to this product.
 */
pub async fn ensure_product_item(
    conn: &mut PgConnection,
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
    require_active: bool,
) -> Result<(), ApiError> {
    let product: Option<(String, bool)> = sqlx::query_as(
        "SELECT pt.identity_mode::text, p.is_active
         FROM atlas.products p
         JOIN atlas.product_types pt ON pt.id = p.product_type_id
         WHERE p.id = $1 AND p.deleted_at IS NULL",
    )
    .bind(product_id)
    .fetch_optional(&mut *conn)
    .await?;
    let (identity_mode, is_active) =
        product.ok_or_else(|| ApiError::NotFound("Product not found.".to_string()))?;
    if require_active && !is_active {
        return Err(ApiError::conflict_with_code(
            "This product is not active.",
            "PRODUCT_INACTIVE",
        ));
    }

    match (identity_mode.as_str(), stock_item_id) {
        ("by_product", None) => Ok(()),
        ("by_product", Some(_)) => Err(ApiError::bad_request_with_code(
            "this product is not tracked by batch or serial",
            "STOCK_ITEM_NOT_ALLOWED",
        )),
        (_, None) => Err(ApiError::bad_request_with_code(
            "this product is tracked by batch or serial; stock_item_id is required",
            "STOCK_ITEM_REQUIRED",
        )),
        (_, Some(item_id)) => {
            let belongs: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM atlas.stock_items WHERE id = $1 AND product_id = $2",
            )
            .bind(item_id)
            .bind(product_id)
            .fetch_optional(conn)
            .await?;
            belongs
                .map(|_| ())
                .ok_or_else(|| ApiError::NotFound("Stock item not found.".to_string()))
        }
    }
}

// === Balance + ledger writes (the only place stock_levels is written)

/**
 * Takes row locks on the balance rows of `store_ids` in a fixed order
 * (store id ascending). Two transactions that touch the same rows then
 * always queue in the same order, so opposite transfers (A to B and B to
 * A at once) wait for each other instead of deadlocking. Rows that do not
 * exist yet lock nothing, which is fine: there is nothing to oversell.
 */
pub async fn lock_levels(
    conn: &mut PgConnection,
    store_ids: &[Uuid],
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
) -> Result<(), ApiError> {
    sqlx::query(
        "SELECT 1 FROM atlas.stock_levels
         WHERE store_id = ANY($1) AND product_id = $2
           AND item_key = coalesce($3, '00000000-0000-0000-0000-000000000000'::uuid)
         ORDER BY store_id
         FOR UPDATE",
    )
    .bind(store_ids)
    .bind(product_id)
    .bind(stock_item_id)
    .fetch_all(conn)
    .await?;
    Ok(())
}

async fn insert_movement(
    conn: &mut PgConnection,
    actor: Actor,
    entry: &Entry<'_>,
    signed_quantity: Decimal,
) -> Result<StockMovement, ApiError> {
    sqlx::query_as::<_, StockMovement>(&format!(
        "INSERT INTO atlas.stock_movements
            (id, tenant_id, store_id, product_id, stock_item_id, movement_type, quantity, reason, reference_id, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6::atlas.stock_movement_type, $7, $8, $9, $10)
         RETURNING {MOVEMENT_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(actor.tenant_id)
    .bind(entry.store_id)
    .bind(entry.product_id)
    .bind(entry.stock_item_id)
    .bind(entry.movement_type)
    .bind(signed_quantity)
    .bind(entry.reason)
    .bind(entry.reference_id)
    .bind(actor.user_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

/** A product is "listed" at a store the first time stock reaches it. */
async fn ensure_listing(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO atlas.product_store_listings (id, tenant_id, product_id, store_id)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (product_id, store_id) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(store_id)
    .execute(conn)
    .await?;
    Ok(())
}

/**
 * Adds `entry.quantity` to the store's balance (creating the row if the
 * product has never been at this store) and writes the matching positive
 * ledger row.
 */
pub async fn credit(
    conn: &mut PgConnection,
    actor: Actor,
    entry: &Entry<'_>,
) -> Result<StockMovement, ApiError> {
    require_positive(entry.quantity)?;
    ensure_listing(conn, actor.tenant_id, entry.store_id, entry.product_id).await?;

    sqlx::query(
        "INSERT INTO atlas.stock_levels (tenant_id, store_id, product_id, stock_item_id, quantity)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (store_id, product_id, item_key) DO UPDATE
         SET quantity = atlas.stock_levels.quantity + EXCLUDED.quantity, updated_at = now()",
    )
    .bind(actor.tenant_id)
    .bind(entry.store_id)
    .bind(entry.product_id)
    .bind(entry.stock_item_id)
    .bind(entry.quantity)
    .execute(&mut *conn)
    .await?;

    insert_movement(conn, actor, entry, entry.quantity).await
}

/**
 * Removes `entry.quantity` from the store's balance only if enough is
 * there. The check and the decrement are one `UPDATE ... WHERE quantity >=`
 * statement, so two concurrent sales cannot both pass the check: the
 * second blocks on the row lock, then re-evaluates against the first one's
 * result and matches zero rows. Zero rows affected covers both "not
 * enough" and "no balance row at all" - a missing record is never treated
 * as sufficient stock.
 */
pub async fn debit(
    conn: &mut PgConnection,
    actor: Actor,
    entry: &Entry<'_>,
) -> Result<StockMovement, ApiError> {
    require_positive(entry.quantity)?;

    let result = sqlx::query(
        "UPDATE atlas.stock_levels
         SET quantity = quantity - $4, updated_at = now()
         WHERE store_id = $1 AND product_id = $2
           AND item_key = coalesce($3, '00000000-0000-0000-0000-000000000000'::uuid)
           AND quantity >= $4",
    )
    .bind(entry.store_id)
    .bind(entry.product_id)
    .bind(entry.stock_item_id)
    .bind(entry.quantity)
    .execute(&mut *conn)
    .await?;

    if result.rows_affected() != 1 {
        return Err(ApiError::conflict_with_code(
            "Insufficient stock at this store.",
            "INSUFFICIENT_STOCK",
        ));
    }

    insert_movement(conn, actor, entry, -entry.quantity).await
}

// === Operations

/**
 * First stock for a product at a store. Fails with `INVENTORY_EXISTS` if
 * the store already has a balance row for it - later changes go through
 * restock, adjustment, transfer or sale, never a second "initial" stock.
 */
pub async fn initial_stock(
    conn: &mut PgConnection,
    actor: Actor,
    payload: &InitialStockRequest,
) -> Result<Option<StockMovement>, ApiError> {
    if payload.quantity < Decimal::ZERO {
        return Err(ApiError::bad_request_with_code(
            "quantity cannot be negative",
            "INVALID_QUANTITY",
        ));
    }
    ensure_store(conn, payload.store_id, true).await?;
    ensure_product_item(conn, payload.product_id, payload.stock_item_id, true).await?;
    ensure_listing(conn, actor.tenant_id, payload.store_id, payload.product_id).await?;

    let inserted = sqlx::query(
        "INSERT INTO atlas.stock_levels (tenant_id, store_id, product_id, stock_item_id, quantity)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (store_id, product_id, item_key) DO NOTHING",
    )
    .bind(actor.tenant_id)
    .bind(payload.store_id)
    .bind(payload.product_id)
    .bind(payload.stock_item_id)
    .bind(payload.quantity)
    .execute(&mut *conn)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(ApiError::conflict_with_code(
            "This product already has inventory at this store.",
            "INVENTORY_EXISTS",
        ));
    }

    // Zero changes nothing, so there is nothing for the ledger to record.
    if payload.quantity == Decimal::ZERO {
        return Ok(None);
    }

    let entry = Entry {
        store_id: payload.store_id,
        product_id: payload.product_id,
        stock_item_id: payload.stock_item_id,
        movement_type: "initial_stock",
        quantity: payload.quantity,
        reason: "Initial stock",
        reference_id: None,
    };
    insert_movement(conn, actor, &entry, payload.quantity)
        .await
        .map(Some)
}

pub async fn restock(
    conn: &mut PgConnection,
    actor: Actor,
    payload: &RestockRequest,
) -> Result<StockMovement, ApiError> {
    require_positive(payload.quantity)?;
    ensure_store(conn, payload.store_id, true).await?;
    ensure_product_item(conn, payload.product_id, payload.stock_item_id, true).await?;

    let reason = match payload.reason.as_deref().map(str::trim) {
        Some(reason) if !reason.is_empty() => reason,
        _ => "Supplier delivery",
    };
    credit(
        conn,
        actor,
        &Entry {
            store_id: payload.store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "purchase_receipt",
            quantity: payload.quantity,
            reason,
            reference_id: payload.reference_id,
        },
    )
    .await
}

/**
 * A signed correction (stock count, damage, expiry). Always needs a
 * reason. A negative adjustment goes through `debit`, so it cannot push a
 * balance below zero either.
 */
pub async fn adjust_stock(
    conn: &mut PgConnection,
    actor: Actor,
    payload: &AdjustStockRequest,
) -> Result<StockMovement, ApiError> {
    if payload.quantity == Decimal::ZERO {
        return Err(ApiError::bad_request_with_code(
            "quantity must not be zero",
            "INVALID_QUANTITY",
        ));
    }
    let reason = require_reason(&payload.reason)?;
    ensure_store(conn, payload.store_id, false).await?;
    ensure_product_item(conn, payload.product_id, payload.stock_item_id, false).await?;

    let entry = Entry {
        store_id: payload.store_id,
        product_id: payload.product_id,
        stock_item_id: payload.stock_item_id,
        movement_type: "adjustment",
        quantity: payload.quantity.abs(),
        reason,
        reference_id: None,
    };
    if payload.quantity > Decimal::ZERO {
        credit(conn, actor, &entry).await
    } else {
        debit(conn, actor, &entry).await
    }
}

/**
 * Two movements in one transaction: `transfer_out` at the source store
 * (negative), `transfer_in` at the destination (positive), sharing one
 * `reference_id` so they trace as a single transfer. The source debit is
 * checked, so the destination can only ever receive what the source
 * actually had: stock is moved, never created or destroyed.
 */
pub async fn transfer_stock(
    conn: &mut PgConnection,
    actor: Actor,
    payload: &TransferStockRequest,
) -> Result<(StockMovement, StockMovement), ApiError> {
    require_positive(payload.quantity)?;
    if payload.from_store_id == payload.to_store_id {
        return Err(ApiError::bad_request_with_code(
            "from_store_id and to_store_id must differ",
            "SAME_STORE",
        ));
    }
    let reason = require_reason(&payload.reason)?;
    ensure_store(conn, payload.from_store_id, true).await?;
    ensure_store(conn, payload.to_store_id, true).await?;
    ensure_product_item(conn, payload.product_id, payload.stock_item_id, true).await?;

    lock_levels(
        conn,
        &[payload.from_store_id, payload.to_store_id],
        payload.product_id,
        payload.stock_item_id,
    )
    .await?;

    let reference_id = Uuid::now_v7();
    let out_movement = debit(
        conn,
        actor,
        &Entry {
            store_id: payload.from_store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "transfer_out",
            quantity: payload.quantity,
            reason,
            reference_id: Some(reference_id),
        },
    )
    .await?;
    let in_movement = credit(
        conn,
        actor,
        &Entry {
            store_id: payload.to_store_id,
            product_id: payload.product_id,
            stock_item_id: payload.stock_item_id,
            movement_type: "transfer_in",
            quantity: payload.quantity,
            reason,
            reference_id: Some(reference_id),
        },
    )
    .await?;

    Ok((out_movement, in_movement))
}

// === Reads

/**
 * The stores in this tenant where `user_id` holds `permission`, by the
 * same grant rules as `roles::service::assert_permission`. Read endpoints
 * that span stores filter to this set, so a store-scoped clerk sees their
 * own branch and not a sibling's.
 */
pub async fn viewable_store_ids(
    conn: &mut PgConnection,
    actor: Actor,
    permission: &str,
) -> Result<Vec<Uuid>, ApiError> {
    sqlx::query_scalar(
        "SELECT s.id
         FROM atlas.stores s
         WHERE s.tenant_id = $2 AND s.deleted_at IS NULL
           AND EXISTS (
                SELECT 1
                FROM atlas.role_grants rg
                JOIN atlas.roles r ON r.id = rg.role_id
                WHERE rg.user_id = $1
                  AND rg.status = 'active'
                  AND $3::atlas.permission = ANY(r.permissions)
                  AND (
                        rg.scope = 'platform'
                     OR (rg.scope = 'tenant' AND rg.tenant_id = $2)
                     OR (rg.scope = 'store' AND rg.store_id = s.id)
                     OR EXISTS (
                          SELECT 1 FROM atlas.store_group_grants sgg
                          WHERE sgg.role_grant_id = rg.id AND sgg.store_id = s.id
                        )
                  )
           )
         ORDER BY s.created_at",
    )
    .bind(actor.user_id)
    .bind(actor.tenant_id)
    .bind(permission)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

/**
 * On-hand for one store. On-hand is `SUM(quantity)` over the store's
 * balance rows (several rows per product when it is tracked by batch or
 * serial). `low_stock` keeps products at or below the listing's
 * `low_stock_threshold`; `max_quantity` is an ad-hoc ceiling.
 */
pub async fn on_hand_for_store(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Option<Uuid>,
    low_stock: bool,
    max_quantity: Option<Decimal>,
) -> Result<Vec<StockLevel>, ApiError> {
    sqlx::query_as::<_, StockLevel>(
        "SELECT sl.product_id, p.name AS product_name, SUM(sl.quantity) AS on_hand,
                COALESCE(l.low_stock_threshold, 0) AS low_stock_threshold
         FROM atlas.stock_levels sl
         JOIN atlas.products p ON p.id = sl.product_id
         LEFT JOIN atlas.product_store_listings l
                ON l.product_id = sl.product_id AND l.store_id = sl.store_id
         WHERE sl.store_id = $1 AND ($2::uuid IS NULL OR sl.product_id = $2)
         GROUP BY sl.product_id, p.name, l.low_stock_threshold
         HAVING (NOT $3 OR SUM(sl.quantity) <= COALESCE(l.low_stock_threshold, 0))
            AND ($4::numeric IS NULL OR SUM(sl.quantity) <= $4)
         ORDER BY p.name, sl.product_id",
    )
    .bind(store_id)
    .bind(product_id)
    .bind(low_stock)
    .bind(max_quantity)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
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
 * Products with their quantity at each of `store_ids`. `total_quantity` is
 * summed here from the per-store rows, never read from storage. With
 * `low_stock`, only locations at or below their threshold are listed (and
 * so only they count toward the total).
 */
pub async fn inventory_by_product(
    conn: &mut PgConnection,
    store_ids: &[Uuid],
    product_id: Option<Uuid>,
    low_stock: bool,
) -> Result<Vec<ProductInventory>, ApiError> {
    let rows = sqlx::query_as::<_, LocationRow>(
        "SELECT sl.product_id, p.name AS product_name, sl.store_id, s.name AS store_name,
                SUM(sl.quantity) AS quantity
         FROM atlas.stock_levels sl
         JOIN atlas.products p ON p.id = sl.product_id
         JOIN atlas.stores s ON s.id = sl.store_id
         LEFT JOIN atlas.product_store_listings l
                ON l.product_id = sl.product_id AND l.store_id = sl.store_id
         WHERE sl.store_id = ANY($1) AND ($2::uuid IS NULL OR sl.product_id = $2)
         GROUP BY sl.product_id, p.name, sl.store_id, s.name, s.created_at, l.low_stock_threshold
         HAVING (NOT $3 OR SUM(sl.quantity) <= COALESCE(l.low_stock_threshold, 0))
         ORDER BY p.name, sl.product_id, s.created_at, sl.store_id",
    )
    .bind(store_ids)
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
    Ok(products)
}

/** One product across `store_ids`; unknown product is a 404, not an empty list. */
pub async fn product_inventory(
    conn: &mut PgConnection,
    store_ids: &[Uuid],
    product_id: Uuid,
) -> Result<ProductInventory, ApiError> {
    let name: Option<String> =
        sqlx::query_scalar("SELECT name FROM atlas.products WHERE id = $1 AND deleted_at IS NULL")
            .bind(product_id)
            .fetch_optional(&mut *conn)
            .await?;
    let name = name.ok_or_else(|| ApiError::NotFound("Product not found.".to_string()))?;

    let found = inventory_by_product(conn, store_ids, Some(product_id), false).await?;
    Ok(found
        .into_iter()
        .next()
        .unwrap_or_else(|| ProductInventory {
            product_id,
            product_name: name,
            total_quantity: Decimal::ZERO,
            locations: Vec::new(),
        }))
}

/** Newest first, capped; filtered to stores the caller may view. */
pub async fn list_movements(
    conn: &mut PgConnection,
    store_ids: &[Uuid],
    store_id: Option<Uuid>,
    product_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<StockMovement>, ApiError> {
    sqlx::query_as::<_, StockMovement>(&format!(
        "SELECT {MOVEMENT_COLUMNS}
         FROM atlas.stock_movements
         WHERE store_id = ANY($1)
           AND ($2::uuid IS NULL OR store_id = $2)
           AND ($3::uuid IS NULL OR product_id = $3)
         ORDER BY created_at DESC, id DESC
         LIMIT $4"
    ))
    .bind(store_ids)
    .bind(store_id)
    .bind(product_id)
    .bind(limit)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}
