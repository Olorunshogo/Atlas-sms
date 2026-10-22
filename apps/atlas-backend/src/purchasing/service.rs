use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};

use super::models::{
    CreatePurchaseOrderRequest, GoodsReceivedNoteData, PurchaseOrder, PurchaseOrderLine, ReceiveGoodsRequest,
    Supplier, UpdateSupplierRequest,
};

// === Suppliers

const SUPPLIER_COLUMNS: &str = "id, name, phone, email, balance_amount_minor";

pub async fn list_suppliers(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<Supplier>, ApiError> {
    return sqlx::query_as::<_, Supplier>(&format!(
        "SELECT {} FROM atlas.suppliers WHERE tenant_id = $1 AND deleted_at IS NULL ORDER BY name",
        SUPPLIER_COLUMNS
    ))
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_supplier(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    name: &str,
    phone: Option<&str>,
    email: Option<&str>,
) -> Result<Supplier, ApiError> {
    return sqlx::query_as::<_, Supplier>(&format!(
        "INSERT INTO atlas.suppliers (id, tenant_id, name, phone, email)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {}",
        SUPPLIER_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .bind(phone)
    .bind(email)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn update_supplier(
    conn: &mut PgConnection,
    supplier_id: Uuid,
    payload: &UpdateSupplierRequest,
) -> Result<Supplier, ApiError> {
    return sqlx::query_as::<_, Supplier>(&format!(
        "UPDATE atlas.suppliers
         SET name = COALESCE($2, name), phone = COALESCE($3, phone), email = COALESCE($4, email)
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {}",
        SUPPLIER_COLUMNS
    ))
    .bind(supplier_id)
    .bind(&payload.name)
    .bind(&payload.phone)
    .bind(&payload.email)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Supplier not found.".to_string()));
}

// === Purchase orders

pub async fn create_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    created_by_user_id: Uuid,
    payload: &CreatePurchaseOrderRequest,
) -> Result<PurchaseOrder, ApiError> {
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a purchase order must have at least one line"));
    }

    let order = sqlx::query_as::<_, PurchaseOrder>(
        "INSERT INTO atlas.purchase_orders (id, tenant_id, store_id, supplier_id, status, created_by_user_id)
         VALUES ($1, $2, $3, $4, 'draft', $5)
         RETURNING id, store_id, supplier_id, status::text AS status",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(payload.supplier_id)
    .bind(created_by_user_id)
    .fetch_one(&mut *conn)
    .await?;

    for line in &payload.lines {
        sqlx::query(
            "INSERT INTO atlas.purchase_order_lines
                (id, tenant_id, purchase_order_id, product_id, quantity_ordered, unit_cost_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(order.id)
        .bind(line.product_id)
        .bind(line.quantity_ordered)
        .bind(line.unit_cost_amount_minor)
        .execute(&mut *conn)
        .await?;
    }

    return Ok(order);
}

pub async fn list_purchase_orders(
    conn: &mut PgConnection,
    store_id: Uuid,
) -> Result<Vec<PurchaseOrder>, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(
        "SELECT id, store_id, supplier_id, status::text AS status
         FROM atlas.purchase_orders WHERE store_id = $1 ORDER BY created_at DESC",
    )
    .bind(store_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn list_purchase_order_lines(
    conn: &mut PgConnection,
    purchase_order_id: Uuid,
) -> Result<Vec<PurchaseOrderLine>, ApiError> {
    return sqlx::query_as::<_, PurchaseOrderLine>(
        "SELECT id, purchase_order_id, product_id, quantity_ordered, quantity_received, unit_cost_amount_minor
         FROM atlas.purchase_order_lines WHERE purchase_order_id = $1 ORDER BY id",
    )
    .bind(purchase_order_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn purchase_order_store_id(conn: &mut PgConnection, purchase_order_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.purchase_orders WHERE id = $1")
        .bind(purchase_order_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown purchase order."));
}

// === Goods received notes

/**
 * Receiving a GRN is the one write path that turns a purchase order into real stock: per line, it writes a `stock_movements` row (`purchase_receipt`), advances that line's `quantity_received`, and adds the line's cost to the supplier's balance - then recomputes the order's own status from all of its lines.
 * A line can never receive more than it has left to receive (`quantity_ordered - quantity_received` so far) - a short shipment is fine and expected, an over-shipment against a specific line is rejected rather than silently accepted; a genuine substitution is a new line on a new order, not an over-receipt on this one.
 */
pub async fn receive_goods(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    purchase_order_id: Uuid,
    received_by_user_id: Uuid,
    payload: &ReceiveGoodsRequest,
) -> Result<GoodsReceivedNoteData, ApiError> {
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a goods received note must have at least one line"));
    }

    let store_id = purchase_order_store_id(conn, purchase_order_id).await?;

    let grn_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.goods_received_notes (id, tenant_id, purchase_order_id, received_by_user_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(grn_id)
    .bind(tenant_id)
    .bind(purchase_order_id)
    .bind(received_by_user_id)
    .execute(&mut *conn)
    .await?;

    for line in &payload.lines {
        let (product_id, quantity_ordered, quantity_received_so_far, unit_cost_amount_minor): (
            Uuid,
            Decimal,
            Decimal,
            i64,
        ) = sqlx::query_as(
            "SELECT product_id, quantity_ordered, quantity_received, unit_cost_amount_minor
             FROM atlas.purchase_order_lines
             WHERE id = $1 AND purchase_order_id = $2",
        )
        .bind(line.purchase_order_line_id)
        .bind(purchase_order_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown purchase order line."))?;

        let remaining = quantity_ordered - quantity_received_so_far;
        if line.quantity_received > remaining {
            return Err(ApiError::bad_request(format!(
                "Cannot receive {} - only {} left on this line.",
                line.quantity_received, remaining
            )));
        }

        let receipt_lines = [PostingLine {
            product_id,
            stock_item_id: None,
            unit_id: None,
            quantity: line.quantity_received,
        }];
        let movements = inventory::service::post(
            conn,
            Posting {
                tenant_id,
                store_id,
                movement_type: MovementType::PurchaseReceipt,
                reason: "Goods received",
                reference_id: Some(grn_id),
                actor_user_id: received_by_user_id,
                policy: NegativeStockPolicy::Allow,
                source: "goods_received",
                lines: &receipt_lines,
            },
        )
        .await?;
        let movement_id = movements[0].id;

        sqlx::query(
            "INSERT INTO atlas.goods_received_note_lines
                (id, tenant_id, goods_received_note_id, purchase_order_line_id, quantity_received, stock_movement_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(grn_id)
        .bind(line.purchase_order_line_id)
        .bind(line.quantity_received)
        .bind(movement_id)
        .execute(&mut *conn)
        .await?;

        sqlx::query("UPDATE atlas.purchase_order_lines SET quantity_received = quantity_received + $2 WHERE id = $1")
            .bind(line.purchase_order_line_id)
            .bind(line.quantity_received)
            .execute(&mut *conn)
            .await?;

        let cost_owed_minor = (line.quantity_received * Decimal::from(unit_cost_amount_minor))
            .round()
            .to_i64()
            .ok_or_else(|| ApiError::bad_request("received cost out of range"))?;
        sqlx::query("UPDATE atlas.suppliers SET balance_amount_minor = balance_amount_minor + $2 WHERE id = (SELECT supplier_id FROM atlas.purchase_orders WHERE id = $1)")
            .bind(purchase_order_id)
            .bind(cost_owed_minor)
            .execute(&mut *conn)
            .await?;
    }

    let order = sqlx::query_as::<_, PurchaseOrder>(
        "UPDATE atlas.purchase_orders SET status = (
            CASE
                WHEN NOT EXISTS (
                    SELECT 1 FROM atlas.purchase_order_lines
                    WHERE purchase_order_id = $1 AND quantity_received < quantity_ordered
                ) THEN 'received'
                WHEN EXISTS (
                    SELECT 1 FROM atlas.purchase_order_lines WHERE purchase_order_id = $1 AND quantity_received > 0
                ) THEN 'partially_received'
                ELSE status
            END
        )::atlas.purchase_order_status
         WHERE id = $1
         RETURNING id, store_id, supplier_id, status::text AS status",
    )
    .bind(purchase_order_id)
    .fetch_one(&mut *conn)
    .await?;

    return Ok(GoodsReceivedNoteData { id: grn_id, purchase_order: order });
}
