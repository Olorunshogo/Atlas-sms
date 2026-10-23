use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};

use super::models::{
    CreatePurchaseOrderRequest, EditPurchaseOrderRequest, GoodsReceivedNoteData, PurchaseOrder, PurchaseOrderLine,
    PurchaseReturn, PurchaseReturnLine, ReceiveGoodsRequest, RecordPurchaseReturnRequest, Supplier,
    UpdateSupplierRequest,
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

// `draft` only. `lines`, when given, replaces the order's lines wholesale.
pub async fn edit_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    purchase_order_id: Uuid,
    payload: &EditPurchaseOrderRequest,
) -> Result<PurchaseOrder, ApiError> {
    let order = sqlx::query_as::<_, PurchaseOrder>(
        "UPDATE atlas.purchase_orders
         SET supplier_id = COALESCE($1, supplier_id)
         WHERE id = $2 AND tenant_id = $3 AND status = 'draft'
         RETURNING id, store_id, supplier_id, status::text AS status",
    )
    .bind(payload.supplier_id)
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a draft purchase order can be edited.",
        codes::PURCHASE_ORDER_NOT_DRAFT,
    ))?;

    if let Some(lines) = &payload.lines {
        if lines.is_empty() {
            return Err(ApiError::bad_request("a purchase order must have at least one line"));
        }
        sqlx::query("DELETE FROM atlas.purchase_order_lines WHERE purchase_order_id = $1")
            .bind(purchase_order_id)
            .execute(&mut *conn)
            .await?;
        for line in lines {
            sqlx::query(
                "INSERT INTO atlas.purchase_order_lines
                    (id, tenant_id, purchase_order_id, product_id, quantity_ordered, unit_cost_amount_minor)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::now_v7())
            .bind(tenant_id)
            .bind(purchase_order_id)
            .bind(line.product_id)
            .bind(line.quantity_ordered)
            .bind(line.unit_cost_amount_minor)
            .execute(&mut *conn)
            .await?;
        }
    }

    return Ok(order);
}

/// `draft` -> `ordered` (FR-21). The one submit/approve step this schema's granularity supports - see the migration comment.
pub async fn submit_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    purchase_order_id: Uuid,
) -> Result<PurchaseOrder, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(
        "UPDATE atlas.purchase_orders
         SET status = 'ordered', submitted_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'draft'
         RETURNING id, store_id, supplier_id, status::text AS status",
    )
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a draft purchase order can be submitted.",
        codes::PURCHASE_ORDER_NOT_DRAFT,
    ));
}

/// `draft` or `ordered` -> `cancelled` (FR-21). Not once anything has been received - that stock is already real.
pub async fn cancel_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    purchase_order_id: Uuid,
) -> Result<PurchaseOrder, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(
        "UPDATE atlas.purchase_orders
         SET status = 'cancelled', cancelled_at = now(), cancelled_by_user_id = $1
         WHERE id = $2 AND tenant_id = $3 AND status IN ('draft', 'ordered')
         RETURNING id, store_id, supplier_id, status::text AS status",
    )
    .bind(user_id)
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a draft or ordered purchase order can be cancelled - it has already been received.",
        codes::PURCHASE_ORDER_NOT_CANCELLABLE,
    ));
}

// === Purchase returns (FR-23)

// Caps each line at what its GRN line has received minus what it's already returned.
pub async fn record_purchase_return(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &RecordPurchaseReturnRequest,
) -> Result<PurchaseReturn, ApiError> {
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a purchase return must have at least one line"));
    }

    let (store_id, supplier_id): (Uuid, Uuid) = sqlx::query_as(
        "SELECT store_id, supplier_id FROM atlas.purchase_orders WHERE id = $1 AND tenant_id = $2",
    )
    .bind(payload.purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown purchase order."))?;

    let return_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.purchase_returns (id, tenant_id, store_id, supplier_id, purchase_order_id, reason, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(return_id)
    .bind(tenant_id)
    .bind(store_id)
    .bind(supplier_id)
    .bind(payload.purchase_order_id)
    .bind(&payload.reason)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;

    let mut lines = Vec::with_capacity(payload.lines.len());
    let mut total_cost_owed_minor: i64 = 0;

    for line in &payload.lines {
        let (product_id, quantity_received, quantity_returned_so_far, unit_cost_amount_minor): (
            Uuid,
            Decimal,
            Decimal,
            i64,
        ) = sqlx::query_as(
            "SELECT grn.quantity_received, grn.quantity_returned, pol.product_id, pol.unit_cost_amount_minor
             FROM atlas.goods_received_note_lines grn
             JOIN atlas.purchase_order_lines pol ON pol.id = grn.purchase_order_line_id
             WHERE grn.id = $1 AND pol.purchase_order_id = $2",
        )
        .bind(line.goods_received_note_line_id)
        .bind(payload.purchase_order_id)
        .fetch_optional(&mut *conn)
        .await?
        .map(|(a, b, c, d): (Decimal, Decimal, Uuid, i64)| (c, a, b, d))
        .ok_or_else(|| ApiError::bad_request("Unknown goods received note line for this purchase order."))?;

        let remaining = quantity_received - quantity_returned_so_far;
        if line.quantity > remaining {
            return Err(ApiError::conflict_with_code(
                format!("Cannot return {} - only {} eligible on this receipt line.", line.quantity, remaining),
                codes::RETURN_QUANTITY_EXCEEDED,
            ));
        }

        let posting_lines = [PostingLine {
            product_id,
            stock_item_id: None,
            unit_id: None,
            quantity: line.quantity,
        }];
        let movements = inventory::service::post(
            conn,
            Posting {
                tenant_id,
                store_id,
                movement_type: MovementType::PurchaseReturn,
                reason: &payload.reason,
                reference_id: Some(return_id),
                actor_user_id: user_id,
                policy: NegativeStockPolicy::Reject,
                source: "purchase_return",
                lines: &posting_lines,
            },
        )
        .await?;
        let movement_id = movements[0].id;

        sqlx::query("UPDATE atlas.goods_received_note_lines SET quantity_returned = quantity_returned + $2 WHERE id = $1")
            .bind(line.goods_received_note_line_id)
            .bind(line.quantity)
            .execute(&mut *conn)
            .await?;

        let line_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO atlas.purchase_return_lines
                (id, tenant_id, purchase_return_id, goods_received_note_line_id, product_id, quantity, unit_cost_amount_minor, stock_movement_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(line_id)
        .bind(tenant_id)
        .bind(return_id)
        .bind(line.goods_received_note_line_id)
        .bind(product_id)
        .bind(line.quantity)
        .bind(unit_cost_amount_minor)
        .bind(movement_id)
        .execute(&mut *conn)
        .await?;

        let cost_owed_minor = (line.quantity * Decimal::from(unit_cost_amount_minor))
            .round()
            .to_i64()
            .ok_or_else(|| ApiError::bad_request("returned cost out of range"))?;
        total_cost_owed_minor += cost_owed_minor;

        lines.push(PurchaseReturnLine {
            id: line_id,
            goods_received_note_line_id: line.goods_received_note_line_id,
            product_id,
            quantity: line.quantity,
            unit_cost_amount_minor,
        });
    }

    sqlx::query("UPDATE atlas.suppliers SET balance_amount_minor = balance_amount_minor - $2 WHERE id = $1")
        .bind(supplier_id)
        .bind(total_cost_owed_minor)
        .execute(&mut *conn)
        .await?;

    return Ok(PurchaseReturn {
        id: return_id,
        store_id,
        supplier_id,
        purchase_order_id: payload.purchase_order_id,
        reason: payload.reason.clone(),
        lines,
    });
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
