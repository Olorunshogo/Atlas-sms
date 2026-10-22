use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};

use super::models::{NegativeStockPolicy, Posting, StockMovement};

/**
 * Converts a quantity in any of a product's sale units to its base unit.
 * The base unit itself has factor 1; any other unit must be declared in `product_sale_units`, or the line is rejected rather than silently treated as the base unit (2 bags of 50kg must deduct 100, not 2).
 */
pub async fn to_base_quantity(
    conn: &mut PgConnection,
    product_id: Uuid,
    unit_id: Uuid,
    quantity: Decimal,
) -> Result<Decimal, ApiError> {
    return Ok(quantity * unit_factor(conn, product_id, unit_id).await?);
}

pub async fn unit_factor(conn: &mut PgConnection, product_id: Uuid, unit_id: Uuid) -> Result<Decimal, ApiError> {
    let factor: Option<Decimal> = sqlx::query_scalar(
        "SELECT CASE WHEN p.base_unit_id = $2 THEN 1::numeric ELSE psu.factor_to_base END
         FROM atlas.products p
         LEFT JOIN atlas.product_sale_units psu ON psu.product_id = p.id AND psu.unit_id = $2
         WHERE p.id = $1",
    )
    .bind(product_id)
    .bind(unit_id)
    .fetch_optional(conn)
    .await?
    .flatten();

    return factor.ok_or_else(|| {
        ApiError::bad_request_with_code(
            format!("Unit {} is not a sale unit of product {}.", unit_id, product_id),
            codes::INVALID_UNIT,
        )
    });
}

/// On-hand for one product at one store, or for one stock item (batch/serial) of it when `stock_item_id` is given.
pub async fn on_hand(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
) -> Result<Decimal, ApiError> {
    return sqlx::query_scalar(
        "SELECT COALESCE(SUM(quantity), 0)
         FROM atlas.stock_movements
         WHERE store_id = $1 AND product_id = $2
           AND ($3::uuid IS NULL OR stock_item_id = $3)",
    )
    .bind(store_id)
    .bind(product_id)
    .bind(stock_item_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * Takes the per-(store, product) advisory locks for a posting, in sorted order so two postings that share products can never deadlock on each other.
 * Transaction-scoped: released at commit or rollback. This is what makes the read-check-insert below safe without a mutable balance column to `SELECT ... FOR UPDATE`.
 */
pub async fn lock_products(conn: &mut PgConnection, store_id: Uuid, product_ids: &[Uuid]) -> Result<(), ApiError> {
    let mut sorted = product_ids.to_vec();
    sorted.sort();
    sorted.dedup();
    for product_id in sorted {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("stock:{}:{}", store_id, product_id))
            .execute(&mut *conn)
            .await?;
    }
    return Ok(());
}

/**
 * The one write path into `atlas.stock_movements`.
 * Converts every line to base units, locks the affected products, checks outgoing lines against on-hand under that lock according to `posting.policy`, then writes one ledger row per line. All-or-nothing: a rejected line fails the whole posting.
 */
pub async fn post(conn: &mut PgConnection, posting: Posting<'_>) -> Result<Vec<StockMovement>, ApiError> {
    let mut base_lines = Vec::with_capacity(posting.lines.len());
    for line in posting.lines {
        if line.quantity == Decimal::ZERO
            || (line.quantity < Decimal::ZERO && posting.movement_type != super::MovementType::Adjustment)
        {
            return Err(ApiError::bad_request("stock quantity must be positive"));
        }
        let factor = match line.unit_id {
            Some(unit_id) => unit_factor(conn, line.product_id, unit_id).await?,
            None => Decimal::ONE,
        };
        base_lines.push((line, posting.movement_type.signed(line.quantity * factor)));
    }

    let product_ids: Vec<Uuid> = base_lines.iter().map(|(line, _)| line.product_id).collect();
    lock_products(conn, posting.store_id, &product_ids).await?;

    // Net outgoing quantity per (product, stock item), so two lines of the
    // same product in one sale are checked together, not one at a time.
    let mut outgoing: BTreeMap<(Uuid, Option<Uuid>), Decimal> = BTreeMap::new();
    for (line, signed) in &base_lines {
        if *signed < Decimal::ZERO {
            *outgoing.entry((line.product_id, line.stock_item_id)).or_default() += -*signed;
        }
    }

    let mut shortfalls: BTreeMap<(Uuid, Option<Uuid>), (Decimal, Decimal)> = BTreeMap::new();
    if posting.policy != NegativeStockPolicy::Allow {
        for ((product_id, stock_item_id), requested) in &outgoing {
            let available = on_hand(conn, posting.store_id, *product_id, *stock_item_id).await?;
            if available < *requested {
                if posting.policy == NegativeStockPolicy::Reject {
                    return Err(ApiError::conflict_with_code(
                        format!(
                            "Not enough stock for product {}: requested {}, available {}.",
                            product_id,
                            requested.normalize(),
                            available.normalize()
                        ),
                        codes::PRODUCT_OUT_OF_STOCK,
                    ));
                }
                shortfalls.insert((*product_id, *stock_item_id), (available, *requested));
            }
        }
    }

    let mut movements = Vec::with_capacity(base_lines.len());
    for (line, signed) in base_lines {
        let movement = sqlx::query_as::<_, StockMovement>(
            "INSERT INTO atlas.stock_movements
                (id, tenant_id, store_id, product_id, stock_item_id, movement_type, quantity,
                 unit_id, unit_quantity, reason, reference_id, created_by_user_id)
             VALUES ($1, $2, $3, $4, $5, $6::atlas.stock_movement_type, $7, $8, $9, $10, $11, $12)
             RETURNING id, store_id, product_id, stock_item_id, movement_type::text AS movement_type, quantity, reason",
        )
        .bind(Uuid::now_v7())
        .bind(posting.tenant_id)
        .bind(posting.store_id)
        .bind(line.product_id)
        .bind(line.stock_item_id)
        .bind(posting.movement_type.as_db_str())
        .bind(signed)
        .bind(line.unit_id)
        .bind(line.unit_id.map(|_| line.quantity))
        .bind(posting.reason)
        .bind(posting.reference_id)
        .bind(posting.actor_user_id)
        .fetch_one(&mut *conn)
        .await?;

        // One exception per short (product, item), attached to its first movement.
        if let Some((available, requested)) = shortfalls.remove(&(line.product_id, line.stock_item_id)) {
            sqlx::query(
                "INSERT INTO atlas.stock_exceptions
                    (id, tenant_id, store_id, product_id, stock_item_id, movement_id, on_hand_before, requested, source, reference_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(Uuid::now_v7())
            .bind(posting.tenant_id)
            .bind(posting.store_id)
            .bind(line.product_id)
            .bind(line.stock_item_id)
            .bind(movement.id)
            .bind(available)
            .bind(requested)
            .bind(posting.source)
            .bind(posting.reference_id)
            .execute(&mut *conn)
            .await?;
        }

        movements.push(movement);
    }

    return Ok(movements);
}
