use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::validate::{require_visible_for, required_text};

use super::models::{
    CreatePurchaseOrderRequest, EditPurchaseOrderRequest, GoodsReceivedNoteData, ListPurchaseReturnsQuery, PurchaseOrder,
    PurchaseOrderCharge, PurchaseOrderChargeInput, PurchaseOrderDetail, PurchaseOrderHeader, PurchaseOrderLine,
    PurchaseOrderLineInput, PurchaseReturn, PurchaseReturnLine, ReceiveGoodsRequest, RecordPurchaseReturnRequest,
};
use super::suppliers;

const ORDER_COLUMNS: &str = "id, store_id, supplier_id, status::text AS status";
const LINE_COLUMNS: &str = "id, purchase_order_id, product_id, quantity_ordered, quantity_received, quantity_cancelled,
     GREATEST(quantity_ordered - quantity_received - quantity_cancelled, 0) AS quantity_outstanding, unit_cost_amount_minor";
const RETURN_COLUMNS: &str = "id, store_id, supplier_id, purchase_order_id, reason, status::text AS status,
     created_by_user_id, created_at, confirmed_by_user_id, confirmed_at, cancelled_at";

fn minor(quantity: Decimal, unit_cost: i64, what: &str) -> Result<i64, ApiError> {
    return (quantity * Decimal::from(unit_cost))
        .round()
        .to_i64()
        .ok_or_else(|| ApiError::bad_request(format!("{} out of range", what)));
}

// === Purchase orders

fn validate_lines(lines: &[PurchaseOrderLineInput]) -> Result<(), ApiError> {
    if lines.is_empty() {
        return Err(ApiError::bad_request("a purchase order must have at least one line"));
    }
    if lines.iter().any(|l| l.quantity_ordered <= Decimal::ZERO || l.unit_cost_amount_minor < 0) {
        return Err(ApiError::bad_request_with_code(
            "quantity_ordered must be positive and unit_cost_amount_minor not negative",
            "VALIDATION_FAILED",
        ));
    }
    return Ok(());
}

fn validate_charges(charges: &[PurchaseOrderChargeInput]) -> Result<(), ApiError> {
    for charge in charges {
        required_text("description", &charge.description, 200)?;
        if charge.amount_minor <= 0 {
            return Err(ApiError::bad_request_with_code("a charge's amount_minor must be positive", "VALIDATION_FAILED"));
        }
    }
    return Ok(());
}

async fn write_lines(conn: &mut PgConnection, tenant_id: Uuid, order_id: Uuid, lines: &[PurchaseOrderLineInput]) -> Result<(), ApiError> {
    for line in lines {
        require_visible_for(conn, tenant_id, "products", line.product_id, "Product").await?;
        sqlx::query(
            "INSERT INTO atlas.purchase_order_lines
                (id, tenant_id, purchase_order_id, product_id, quantity_ordered, unit_cost_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(order_id)
        .bind(line.product_id)
        .bind(line.quantity_ordered)
        .bind(line.unit_cost_amount_minor)
        .execute(&mut *conn)
        .await?;
    }
    return Ok(());
}

async fn write_charges(conn: &mut PgConnection, tenant_id: Uuid, order_id: Uuid, charges: &[PurchaseOrderChargeInput]) -> Result<(), ApiError> {
    for charge in charges {
        sqlx::query(
            "INSERT INTO atlas.purchase_order_charges (id, tenant_id, purchase_order_id, description, amount_minor)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(order_id)
        .bind(charge.description.trim())
        .bind(charge.amount_minor)
        .execute(&mut *conn)
        .await?;
    }
    return Ok(());
}

pub async fn create_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    created_by_user_id: Uuid,
    payload: &CreatePurchaseOrderRequest,
) -> Result<PurchaseOrder, ApiError> {
    validate_lines(&payload.lines)?;
    validate_charges(&payload.charges)?;
    require_visible_for(conn, tenant_id, "stores", payload.store_id, "Store").await?;
    require_visible_for(conn, tenant_id, "suppliers", payload.supplier_id, "Supplier").await?;
    suppliers::assert_active(conn, payload.supplier_id).await?;

    let order = sqlx::query_as::<_, PurchaseOrder>(&format!(
        "INSERT INTO atlas.purchase_orders (id, tenant_id, store_id, supplier_id, status, created_by_user_id)
         VALUES ($1, $2, $3, $4, 'draft', $5)
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(payload.supplier_id)
    .bind(created_by_user_id)
    .fetch_one(&mut *conn)
    .await?;

    write_lines(conn, tenant_id, order.id, &payload.lines).await?;
    write_charges(conn, tenant_id, order.id, &payload.charges).await?;
    return Ok(order);
}

pub async fn list_purchase_orders(conn: &mut PgConnection, store_id: Uuid) -> Result<Vec<PurchaseOrder>, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(&format!(
        "SELECT {ORDER_COLUMNS} FROM atlas.purchase_orders WHERE store_id = $1 ORDER BY created_at DESC"
    ))
    .bind(store_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn list_purchase_order_lines(conn: &mut PgConnection, purchase_order_id: Uuid) -> Result<Vec<PurchaseOrderLine>, ApiError> {
    return sqlx::query_as::<_, PurchaseOrderLine>(&format!(
        "SELECT {LINE_COLUMNS} FROM atlas.purchase_order_lines WHERE purchase_order_id = $1 ORDER BY id"
    ))
    .bind(purchase_order_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn get_purchase_order(conn: &mut PgConnection, purchase_order_id: Uuid) -> Result<PurchaseOrderDetail, ApiError> {
    let header = sqlx::query_as::<_, PurchaseOrderHeader>(
        "SELECT id, store_id, supplier_id, status::text AS status, created_by_user_id, created_at,
                submitted_at, approved_by_user_id, approved_at, cancelled_at
         FROM atlas.purchase_orders WHERE id = $1",
    )
    .bind(purchase_order_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Purchase order not found."))?;
    let lines = list_purchase_order_lines(conn, purchase_order_id).await?;
    let charges = sqlx::query_as::<_, PurchaseOrderCharge>(
        "SELECT id, description, amount_minor FROM atlas.purchase_order_charges WHERE purchase_order_id = $1 ORDER BY id",
    )
    .bind(purchase_order_id)
    .fetch_all(&mut *conn)
    .await?;

    let mut lines_total: i64 = 0;
    for line in &lines {
        lines_total += minor(line.quantity_ordered, line.unit_cost_amount_minor, "order total")?;
    }
    let charges_total: i64 = charges.iter().map(|c| c.amount_minor).sum();
    return Ok(PurchaseOrderDetail {
        id: header.id,
        store_id: header.store_id,
        supplier_id: header.supplier_id,
        status: header.status,
        created_by_user_id: header.created_by_user_id,
        created_at: header.created_at,
        submitted_at: header.submitted_at,
        approved_by_user_id: header.approved_by_user_id,
        approved_at: header.approved_at,
        cancelled_at: header.cancelled_at,
        lines,
        charges,
        lines_total_amount_minor: lines_total,
        charges_total_amount_minor: charges_total,
        total_amount_minor: lines_total + charges_total,
    });
}

pub async fn purchase_order_store_id(conn: &mut PgConnection, purchase_order_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.purchase_orders WHERE id = $1")
        .bind(purchase_order_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown purchase order."));
}

/// Before approval only: `draft`, or `submitted`, which goes back to `draft` to be submitted again.
pub async fn edit_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    purchase_order_id: Uuid,
    payload: &EditPurchaseOrderRequest,
) -> Result<PurchaseOrder, ApiError> {
    if let Some(lines) = &payload.lines {
        validate_lines(lines)?;
    }
    if let Some(charges) = &payload.charges {
        validate_charges(charges)?;
    }
    if let Some(supplier_id) = payload.supplier_id {
        require_visible_for(conn, tenant_id, "suppliers", supplier_id, "Supplier").await?;
        suppliers::assert_active(conn, supplier_id).await?;
    }
    let order = sqlx::query_as::<_, PurchaseOrder>(&format!(
        "UPDATE atlas.purchase_orders
         SET supplier_id = COALESCE($1, supplier_id), status = 'draft', submitted_at = NULL
         WHERE id = $2 AND tenant_id = $3 AND status IN ('draft', 'submitted')
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(payload.supplier_id)
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a purchase order that isn't approved yet can be edited.",
        codes::PURCHASE_ORDER_NOT_DRAFT,
    ))?;

    if let Some(lines) = &payload.lines {
        sqlx::query("DELETE FROM atlas.purchase_order_lines WHERE purchase_order_id = $1")
            .bind(purchase_order_id)
            .execute(&mut *conn)
            .await?;
        write_lines(conn, tenant_id, purchase_order_id, lines).await?;
    }
    if let Some(charges) = &payload.charges {
        sqlx::query("DELETE FROM atlas.purchase_order_charges WHERE purchase_order_id = $1")
            .bind(purchase_order_id)
            .execute(&mut *conn)
            .await?;
        write_charges(conn, tenant_id, purchase_order_id, charges).await?;
    }
    return Ok(order);
}

/// `draft` -> `submitted`: waiting for someone with `purchase.approve`.
pub async fn submit_purchase_order(conn: &mut PgConnection, tenant_id: Uuid, purchase_order_id: Uuid) -> Result<PurchaseOrder, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(&format!(
        "UPDATE atlas.purchase_orders SET status = 'submitted', submitted_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'draft'
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a draft purchase order can be submitted.", codes::PURCHASE_ORDER_NOT_DRAFT));
}

/// `submitted` -> `ordered`: placed with the supplier, and from now on receivable.
pub async fn approve_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    purchase_order_id: Uuid,
) -> Result<PurchaseOrder, ApiError> {
    return sqlx::query_as::<_, PurchaseOrder>(&format!(
        "UPDATE atlas.purchase_orders SET status = 'ordered', approved_by_user_id = $1, approved_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status = 'submitted'
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(user_id)
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "Only a submitted purchase order can be approved.",
        codes::PURCHASE_ORDER_NOT_SUBMITTED,
    ));
}

/**
 * Cancels whatever is still outstanding. Before any receipt that is the whole order; after a partial receipt the
 * received stock stays and only the rest is cancelled. A fully received order has nothing left to cancel.
 */
pub async fn cancel_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    purchase_order_id: Uuid,
) -> Result<PurchaseOrder, ApiError> {
    let order = sqlx::query_as::<_, PurchaseOrder>(&format!(
        "UPDATE atlas.purchase_orders
         SET status = 'cancelled', cancelled_at = now(), cancelled_by_user_id = $1
         WHERE id = $2 AND tenant_id = $3 AND status IN ('draft', 'submitted', 'ordered', 'partially_received')
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(user_id)
    .bind(purchase_order_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code(
        "This purchase order is already fully received or cancelled.",
        codes::PURCHASE_ORDER_NOT_CANCELLABLE,
    ))?;

    sqlx::query(
        "UPDATE atlas.purchase_order_lines
         SET quantity_cancelled = GREATEST(quantity_ordered - quantity_received, 0)
         WHERE purchase_order_id = $1",
    )
    .bind(purchase_order_id)
    .execute(&mut *conn)
    .await?;
    return Ok(order);
}

// === Goods received notes

/**
 * Turns an approved order into real stock: per line, a `purchase_receipt` movement, the line's `quantity_received`,
 * and its cost on the supplier's balance; the order's charges ride on the first receipt. Then the order's status
 * is recomputed from its lines.
 * A line can't receive more than is outstanding unless `allow_over_receipt` (the caller holds `purchase.approve`)
 * and the line gives an `over_receipt_reason`, which is kept on the receipt line.
 */
pub async fn receive_goods(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    purchase_order_id: Uuid,
    received_by_user_id: Uuid,
    allow_over_receipt: bool,
    payload: &ReceiveGoodsRequest,
) -> Result<GoodsReceivedNoteData, ApiError> {
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a goods received note must have at least one line"));
    }

    let (store_id, supplier_id, status, charges_posted): (Uuid, Uuid, String, bool) = sqlx::query_as(
        "SELECT store_id, supplier_id, status::text, charges_posted_at IS NOT NULL
         FROM atlas.purchase_orders WHERE id = $1 FOR UPDATE",
    )
    .bind(purchase_order_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown purchase order."))?;
    if status != "ordered" && status != "partially_received" {
        return Err(ApiError::conflict_with_code(
            "Only an approved purchase order can be received.",
            codes::PURCHASE_ORDER_NOT_RECEIVABLE,
        ));
    }

    let charges_minor: i64 = if charges_posted {
        0
    } else {
        sqlx::query_scalar("SELECT COALESCE(SUM(amount_minor), 0)::bigint FROM atlas.purchase_order_charges WHERE purchase_order_id = $1")
            .bind(purchase_order_id)
            .fetch_one(&mut *conn)
            .await?
    };

    let grn_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.goods_received_notes (id, tenant_id, purchase_order_id, received_by_user_id, charges_amount_minor)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(grn_id)
    .bind(tenant_id)
    .bind(purchase_order_id)
    .bind(received_by_user_id)
    .bind(charges_minor)
    .execute(&mut *conn)
    .await?;

    let mut owed_minor = charges_minor;
    for line in &payload.lines {
        if line.quantity_received <= Decimal::ZERO {
            return Err(ApiError::bad_request("quantity_received must be positive"));
        }
        let (product_id, outstanding, unit_cost_amount_minor): (Uuid, Decimal, i64) = sqlx::query_as(
            "SELECT product_id, GREATEST(quantity_ordered - quantity_received - quantity_cancelled, 0), unit_cost_amount_minor
             FROM atlas.purchase_order_lines
             WHERE id = $1 AND purchase_order_id = $2",
        )
        .bind(line.purchase_order_line_id)
        .bind(purchase_order_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown purchase order line."))?;

        let over_reason = line.over_receipt_reason.as_deref().map(str::trim).filter(|r| !r.is_empty());
        if line.quantity_received > outstanding && !(allow_over_receipt && over_reason.is_some()) {
            return Err(ApiError::conflict_with_code(
                format!(
                    "Cannot receive {} - only {} outstanding on this line. An approver can receive more with an over_receipt_reason.",
                    line.quantity_received.normalize(),
                    outstanding.normalize()
                ),
                codes::RECEIPT_EXCEEDS_OUTSTANDING,
            ));
        }
        let recorded_reason = if line.quantity_received > outstanding { over_reason } else { None };

        let stock_item_id = match line.batch_number.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
            Some(batch_number) => {
                if !inventory::batches::tracks_batches(conn, product_id).await? {
                    return Err(ApiError::bad_request_with_code("This product isn't batch-tracked; leave batch_number out.", "VALIDATION_FAILED"));
                }
                Some(inventory::batches::find_or_create_batch(conn, tenant_id, product_id, batch_number, line.expiry_date).await?)
            }
            None => {
                inventory::batches::check_batch_given(conn, product_id, None).await?;
                None
            }
        };
        let receipt_lines = [PostingLine {
            product_id,
            stock_item_id,
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

        sqlx::query(
            "INSERT INTO atlas.goods_received_note_lines
                (id, tenant_id, goods_received_note_id, purchase_order_line_id, quantity_received, stock_movement_id, over_receipt_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(grn_id)
        .bind(line.purchase_order_line_id)
        .bind(line.quantity_received)
        .bind(movements[0].id)
        .bind(recorded_reason)
        .execute(&mut *conn)
        .await?;

        sqlx::query("UPDATE atlas.purchase_order_lines SET quantity_received = quantity_received + $2 WHERE id = $1")
            .bind(line.purchase_order_line_id)
            .bind(line.quantity_received)
            .execute(&mut *conn)
            .await?;

        owed_minor += minor(line.quantity_received, unit_cost_amount_minor, "received cost")?;
    }

    sqlx::query("UPDATE atlas.suppliers SET balance_amount_minor = balance_amount_minor + $2 WHERE id = $1")
        .bind(supplier_id)
        .bind(owed_minor)
        .execute(&mut *conn)
        .await?;

    let order = sqlx::query_as::<_, PurchaseOrder>(&format!(
        "UPDATE atlas.purchase_orders SET
            charges_posted_at = COALESCE(charges_posted_at, now()),
            status = (
                CASE
                    WHEN NOT EXISTS (
                        SELECT 1 FROM atlas.purchase_order_lines
                        WHERE purchase_order_id = $1 AND quantity_received < quantity_ordered
                    ) THEN 'received'
                    ELSE 'partially_received'
                END
            )::atlas.purchase_order_status
         WHERE id = $1
         RETURNING {ORDER_COLUMNS}"
    ))
    .bind(purchase_order_id)
    .fetch_one(&mut *conn)
    .await?;

    return Ok(GoodsReceivedNoteData { id: grn_id, purchase_order: order });
}

// === Purchase returns (FR-23)

/// Received less returned less what pending returns already claim, so two pending returns can't share the same units.
async fn returnable(conn: &mut PgConnection, grn_line_id: Uuid, excluding_return: Option<Uuid>) -> Result<Decimal, ApiError> {
    return sqlx::query_scalar(
        "SELECT grn.quantity_received - grn.quantity_returned - COALESCE((
                    SELECT SUM(rl.quantity) FROM atlas.purchase_return_lines rl
                    JOIN atlas.purchase_returns r ON r.id = rl.purchase_return_id
                    WHERE rl.goods_received_note_line_id = grn.id AND r.status = 'pending'
                      AND ($2::uuid IS NULL OR r.id <> $2)
                ), 0)
         FROM atlas.goods_received_note_lines grn WHERE grn.id = $1",
    )
    .bind(grn_line_id)
    .bind(excluding_return)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn get_purchase_return(conn: &mut PgConnection, return_id: Uuid) -> Result<PurchaseReturn, ApiError> {
    let mut purchase_return = sqlx::query_as::<_, PurchaseReturn>(&format!(
        "SELECT {RETURN_COLUMNS} FROM atlas.purchase_returns WHERE id = $1"
    ))
    .bind(return_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Purchase return not found."))?;
    purchase_return.lines = sqlx::query_as::<_, PurchaseReturnLine>(
        "SELECT id, goods_received_note_line_id, product_id, quantity, unit_cost_amount_minor,
                ROUND(quantity * unit_cost_amount_minor)::bigint AS amount_minor
         FROM atlas.purchase_return_lines WHERE purchase_return_id = $1 ORDER BY id",
    )
    .bind(return_id)
    .fetch_all(conn)
    .await?;
    purchase_return.total_amount_minor = purchase_return.lines.iter().map(|l| l.amount_minor).sum();
    return Ok(purchase_return);
}

pub async fn list_purchase_returns(conn: &mut PgConnection, query: &ListPurchaseReturnsQuery) -> Result<Vec<PurchaseReturn>, ApiError> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM atlas.purchase_returns
         WHERE store_id = $1 AND ($2::uuid IS NULL OR supplier_id = $2)
         ORDER BY created_at DESC",
    )
    .bind(query.store_id)
    .bind(query.supplier_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut returns = Vec::with_capacity(ids.len());
    for id in ids {
        returns.push(get_purchase_return(conn, id).await?);
    }
    return Ok(returns);
}

/// Records a `pending` return against a receipt. Nothing moves until it is confirmed.
pub async fn record_purchase_return(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &RecordPurchaseReturnRequest,
) -> Result<PurchaseReturn, ApiError> {
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a purchase return must have at least one line"));
    }
    let reason = required_text("reason", &payload.reason, 500)?;

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
        "INSERT INTO atlas.purchase_returns (id, tenant_id, store_id, supplier_id, purchase_order_id, reason, created_by_user_id, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')",
    )
    .bind(return_id)
    .bind(tenant_id)
    .bind(store_id)
    .bind(supplier_id)
    .bind(payload.purchase_order_id)
    .bind(&reason)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;

    for line in &payload.lines {
        if line.quantity <= Decimal::ZERO {
            return Err(ApiError::bad_request("quantity must be positive"));
        }
        let (product_id, unit_cost_amount_minor): (Uuid, i64) = sqlx::query_as(
            "SELECT pol.product_id, pol.unit_cost_amount_minor
             FROM atlas.goods_received_note_lines grn
             JOIN atlas.purchase_order_lines pol ON pol.id = grn.purchase_order_line_id
             WHERE grn.id = $1 AND pol.purchase_order_id = $2",
        )
        .bind(line.goods_received_note_line_id)
        .bind(payload.purchase_order_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown goods received note line for this purchase order."))?;

        let remaining = returnable(conn, line.goods_received_note_line_id, Some(return_id)).await?;
        if line.quantity > remaining {
            return Err(ApiError::conflict_with_code(
                format!("Cannot return {} - only {} eligible on this receipt line.", line.quantity.normalize(), remaining.normalize()),
                codes::RETURN_QUANTITY_EXCEEDED,
            ));
        }

        sqlx::query(
            "INSERT INTO atlas.purchase_return_lines
                (id, tenant_id, purchase_return_id, goods_received_note_line_id, product_id, quantity, unit_cost_amount_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(return_id)
        .bind(line.goods_received_note_line_id)
        .bind(product_id)
        .bind(line.quantity)
        .bind(unit_cost_amount_minor)
        .execute(&mut *conn)
        .await?;
    }

    return get_purchase_return(conn, return_id).await;
}

/**
 * `pending` -> `confirmed`: takes the goods out of the batch they arrived in and off the supplier's balance.
 * Quantities are checked again, since stock or other returns may have moved since it was recorded.
 */
pub async fn confirm_purchase_return(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    return_id: Uuid,
) -> Result<PurchaseReturn, ApiError> {
    let claimed: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "UPDATE atlas.purchase_returns SET status = 'confirmed', confirmed_by_user_id = $1, confirmed_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status = 'pending'
         RETURNING store_id, supplier_id, reason",
    )
    .bind(user_id)
    .bind(return_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?;
    let (store_id, supplier_id, reason) = claimed.ok_or_else(|| {
        ApiError::conflict_with_code("Only a pending purchase return can be confirmed or cancelled.", codes::PURCHASE_RETURN_NOT_PENDING)
    })?;

    let pending = get_purchase_return(conn, return_id).await?;
    let mut total_minor: i64 = 0;
    for line in &pending.lines {
        let remaining = returnable(conn, line.goods_received_note_line_id, Some(return_id)).await?;
        if line.quantity > remaining {
            return Err(ApiError::conflict_with_code(
                format!("Cannot return {} - only {} eligible on this receipt line.", line.quantity.normalize(), remaining.normalize()),
                codes::RETURN_QUANTITY_EXCEEDED,
            ));
        }
        let received_stock_item_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT m.stock_item_id FROM atlas.goods_received_note_lines grn
             LEFT JOIN atlas.stock_movements m ON m.id = grn.stock_movement_id
             WHERE grn.id = $1",
        )
        .bind(line.goods_received_note_line_id)
        .fetch_one(&mut *conn)
        .await?;
        let posting_lines = [PostingLine {
            product_id: line.product_id,
            stock_item_id: received_stock_item_id,
            unit_id: None,
            quantity: line.quantity,
        }];
        let movements = inventory::service::post(
            conn,
            Posting {
                tenant_id,
                store_id,
                movement_type: MovementType::PurchaseReturn,
                reason: &reason,
                reference_id: Some(return_id),
                actor_user_id: user_id,
                policy: NegativeStockPolicy::Reject,
                source: "purchase_return",
                lines: &posting_lines,
            },
        )
        .await?;
        sqlx::query("UPDATE atlas.purchase_return_lines SET stock_movement_id = $2 WHERE id = $1")
            .bind(line.id)
            .bind(movements[0].id)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE atlas.goods_received_note_lines SET quantity_returned = quantity_returned + $2 WHERE id = $1")
            .bind(line.goods_received_note_line_id)
            .bind(line.quantity)
            .execute(&mut *conn)
            .await?;
        total_minor += line.amount_minor;
    }

    sqlx::query("UPDATE atlas.suppliers SET balance_amount_minor = balance_amount_minor - $2 WHERE id = $1")
        .bind(supplier_id)
        .bind(total_minor)
        .execute(&mut *conn)
        .await?;
    return get_purchase_return(conn, return_id).await;
}

/// `pending` -> `cancelled`. Nothing had moved, so nothing moves back.
pub async fn cancel_purchase_return(conn: &mut PgConnection, tenant_id: Uuid, return_id: Uuid) -> Result<PurchaseReturn, ApiError> {
    let updated: Option<Uuid> = sqlx::query_scalar(
        "UPDATE atlas.purchase_returns SET status = 'cancelled', cancelled_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'pending' RETURNING id",
    )
    .bind(return_id)
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await?;
    if updated.is_none() {
        return Err(ApiError::conflict_with_code(
            "Only a pending purchase return can be confirmed or cancelled.",
            codes::PURCHASE_RETURN_NOT_PENDING,
        ));
    }
    return get_purchase_return(conn, return_id).await;
}

pub async fn purchase_return_store_id(conn: &mut PgConnection, return_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.purchase_returns WHERE id = $1")
        .bind(return_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Purchase return not found."));
}
