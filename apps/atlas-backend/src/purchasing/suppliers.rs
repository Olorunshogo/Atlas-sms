/*!
 * FR-20 supplier records: details and payment terms, the products a supplier sells, payments made to them,
 * and their full transaction history. Reads are open to any member; writes need `supplier.manage`.
 * The balance is what the business owes: receipts raise it, returns and payments lower it.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::{codes, ApiError};
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{nothing_to_update, require_visible, required_text};

const SUPPLIER_COLUMNS: &str =
    "id, code, name, phone, email, address, payment_terms_days, balance_amount_minor, deactivated_at";

// === Types

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Supplier {
    pub id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub payment_terms_days: Option<i32>,
    pub balance_amount_minor: i64,
    pub deactivated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SupplierProduct {
    pub product_id: Uuid,
    pub product_name: String,
    pub supplier_sku: Option<String>,
    pub unit_cost_amount_minor: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SupplierDetail {
    #[serde(flatten)]
    pub supplier: Supplier,
    pub products: Vec<SupplierProduct>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSupplierRequest {
    pub name: String,
    pub code: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub payment_terms_days: Option<i32>,
}

/// No `balance_amount_minor`: the balance only moves through receipts, returns and payments.
#[derive(Debug, Deserialize)]
pub struct UpdateSupplierRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub payment_terms_days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ListSuppliersQuery {
    /// Name, code, phone or email, case-insensitive substring.
    pub q: Option<String>,
    #[serde(default)]
    pub include_inactive: bool,
}

#[derive(Debug, Deserialize)]
pub struct SupplierProductInput {
    pub product_id: Uuid,
    pub supplier_sku: Option<String>,
    pub unit_cost_amount_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SetSupplierProductsRequest {
    pub products: Vec<SupplierProductInput>,
}

#[derive(Debug, Deserialize)]
pub struct RecordSupplierPaymentRequest {
    pub amount_minor: i64,
    pub method: String,
    pub reference: Option<String>,
    pub paid_on: NaiveDate,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SupplierPayment {
    pub id: Uuid,
    pub supplier_id: Uuid,
    pub amount_minor: i64,
    pub method: String,
    pub reference: Option<String>,
    pub paid_on: NaiveDate,
    pub recorded_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/**
 * One entry in a supplier's history. `kind` is `purchase_order`, `goods_receipt`, `purchase_return` or `payment`;
 * `balance_effect_minor` is how it moved what is owed (zero for an order, which owes nothing until received).
 */
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SupplierTransaction {
    pub kind: String,
    pub id: Uuid,
    pub purchase_order_id: Option<Uuid>,
    pub store_id: Option<Uuid>,
    pub status: Option<String>,
    pub amount_minor: i64,
    pub balance_effect_minor: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SupplierStatement {
    pub supplier_id: Uuid,
    pub balance_amount_minor: i64,
    pub transactions: Vec<SupplierTransaction>,
}

// === Helpers

async fn assert_manage(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "supplier.manage").await;
}

fn clean(value: &Option<String>) -> Option<String> {
    return value.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
}

fn check_terms(days: Option<i32>) -> Result<(), ApiError> {
    if days.is_some_and(|d| d < 0) {
        return Err(ApiError::bad_request_with_code("payment_terms_days must not be negative", "VALIDATION_FAILED"));
    }
    return Ok(());
}

fn duplicate_code(err: sqlx::Error) -> ApiError {
    return match err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            ApiError::conflict_with_code("Another supplier already uses that code.", "ALREADY_EXISTS")
        }
        other => ApiError::from(other),
    };
}

pub async fn get_supplier(conn: &mut PgConnection, supplier_id: Uuid) -> Result<Supplier, ApiError> {
    return sqlx::query_as::<_, Supplier>(&format!(
        "SELECT {SUPPLIER_COLUMNS} FROM atlas.suppliers WHERE id = $1 AND deleted_at IS NULL"
    ))
    .bind(supplier_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Supplier not found."));
}

/// New orders can only go to an active supplier.
pub async fn assert_active(conn: &mut PgConnection, supplier_id: Uuid) -> Result<(), ApiError> {
    let supplier = get_supplier(conn, supplier_id).await?;
    if supplier.deactivated_at.is_some() {
        return Err(ApiError::conflict_with_code("This supplier is deactivated.", codes::SUPPLIER_INACTIVE));
    }
    return Ok(());
}

async fn products_of(conn: &mut PgConnection, supplier_id: Uuid) -> Result<Vec<SupplierProduct>, ApiError> {
    return sqlx::query_as::<_, SupplierProduct>(
        "SELECT sp.product_id, p.name AS product_name, sp.supplier_sku, sp.unit_cost_amount_minor
         FROM atlas.supplier_products sp JOIN atlas.products p ON p.id = sp.product_id
         WHERE sp.supplier_id = $1 ORDER BY p.name",
    )
    .bind(supplier_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

// === Handlers

/// `GET /api/v1/suppliers?q=&include_inactive=`.
pub async fn list_suppliers(mut tx: TenantTx, Query(query): Query<ListSuppliersQuery>) -> Result<Json<ApiEnvelope<Vec<Supplier>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let search = clean(&query.q);
    let suppliers = sqlx::query_as::<_, Supplier>(&format!(
        "SELECT {SUPPLIER_COLUMNS} FROM atlas.suppliers
         WHERE tenant_id = $1 AND deleted_at IS NULL
           AND ($2 OR deactivated_at IS NULL)
           AND ($3::text IS NULL
                OR name ILIKE '%' || $3 || '%' OR code ILIKE '%' || $3 || '%'
                OR phone ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
         ORDER BY name"
    ))
    .bind(tenant_id)
    .bind(query.include_inactive)
    .bind(search)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Suppliers fetched.", suppliers)));
}

/// `GET /api/v1/suppliers/:id`. The record plus the products it supplies.
pub async fn show_supplier(Path(supplier_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<SupplierDetail>>, ApiError> {
    let supplier = get_supplier(&mut tx, supplier_id).await?;
    let products = products_of(&mut tx, supplier_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier fetched.", SupplierDetail { supplier, products })));
}

/// `POST /api/v1/suppliers`.
pub async fn create_supplier(mut tx: TenantTx, Json(body): Json<CreateSupplierRequest>) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    assert_manage(&mut tx).await?;
    let name = required_text("name", &body.name, 200)?;
    check_terms(body.payment_terms_days)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let supplier = sqlx::query_as::<_, Supplier>(&format!(
        "INSERT INTO atlas.suppliers (id, tenant_id, name, code, phone, email, address, payment_terms_days)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {SUPPLIER_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&name)
    .bind(clean(&body.code))
    .bind(clean(&body.phone))
    .bind(clean(&body.email))
    .bind(clean(&body.address))
    .bind(body.payment_terms_days)
    .fetch_one(&mut *tx)
    .await
    .map_err(duplicate_code)?;
    audit::record(&mut tx, tenant_id, user_id, "supplier", supplier.id, "supplier.create", serde_json::to_value(&supplier).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier added.", supplier)));
}

/// `PATCH /api/v1/suppliers/:id`.
pub async fn update_supplier(
    Path(supplier_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateSupplierRequest>,
) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    assert_manage(&mut tx).await?;
    if body.name.is_none()
        && body.code.is_none()
        && body.phone.is_none()
        && body.email.is_none()
        && body.address.is_none()
        && body.payment_terms_days.is_none()
    {
        return Err(nothing_to_update());
    }
    let name = match body.name.as_deref() {
        Some(name) => Some(required_text("name", name, 200)?),
        None => None,
    };
    check_terms(body.payment_terms_days)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let supplier = sqlx::query_as::<_, Supplier>(&format!(
        "UPDATE atlas.suppliers
         SET name = COALESCE($2, name), code = COALESCE($3, code), phone = COALESCE($4, phone),
             email = COALESCE($5, email), address = COALESCE($6, address),
             payment_terms_days = COALESCE($7, payment_terms_days)
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {SUPPLIER_COLUMNS}"
    ))
    .bind(supplier_id)
    .bind(&name)
    .bind(clean(&body.code))
    .bind(clean(&body.phone))
    .bind(clean(&body.email))
    .bind(clean(&body.address))
    .bind(body.payment_terms_days)
    .fetch_optional(&mut *tx)
    .await
    .map_err(duplicate_code)?
    .ok_or_else(|| ApiError::not_found("Supplier not found."))?;
    audit::record(&mut tx, tenant_id, user_id, "supplier", supplier.id, "supplier.update", serde_json::to_value(&supplier).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier updated.", supplier)));
}

async fn set_active(mut tx: TenantTx, supplier_id: Uuid, active: bool) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    assert_manage(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let supplier = sqlx::query_as::<_, Supplier>(&format!(
        "UPDATE atlas.suppliers SET deactivated_at = CASE WHEN $2 THEN NULL ELSE COALESCE(deactivated_at, now()) END
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {SUPPLIER_COLUMNS}"
    ))
    .bind(supplier_id)
    .bind(active)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Supplier not found."))?;
    let action = if active { "supplier.reactivate" } else { "supplier.deactivate" };
    audit::record(&mut tx, tenant_id, user_id, "supplier", supplier.id, action, serde_json::to_value(&supplier).ok()).await?;
    tx.commit().await?;
    let message = if active { "Supplier reactivated." } else { "Supplier deactivated." };
    return Ok(Json(ApiEnvelope::ok(message, supplier)));
}

/// `POST /api/v1/suppliers/:id/deactivate`. Its history stays; it just can't take new orders.
pub async fn deactivate_supplier(Path(supplier_id): Path<Uuid>, tx: TenantTx) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    return set_active(tx, supplier_id, false).await;
}

/// `POST /api/v1/suppliers/:id/reactivate`.
pub async fn reactivate_supplier(Path(supplier_id): Path<Uuid>, tx: TenantTx) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    return set_active(tx, supplier_id, true).await;
}

/// `PUT /api/v1/suppliers/:id/products`. Replaces the list of products this supplier sells.
pub async fn set_supplier_products(
    Path(supplier_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<SetSupplierProductsRequest>,
) -> Result<Json<ApiEnvelope<Vec<SupplierProduct>>>, ApiError> {
    assert_manage(&mut tx).await?;
    get_supplier(&mut tx, supplier_id).await?;
    if body.products.iter().any(|p| p.unit_cost_amount_minor.is_some_and(|c| c < 0)) {
        return Err(ApiError::bad_request_with_code("unit_cost_amount_minor must not be negative", "VALIDATION_FAILED"));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    sqlx::query("DELETE FROM atlas.supplier_products WHERE supplier_id = $1")
        .bind(supplier_id)
        .execute(&mut *tx)
        .await?;
    for product in &body.products {
        require_visible(&mut tx, "products", product.product_id, "Product").await?;
        sqlx::query(
            "INSERT INTO atlas.supplier_products (supplier_id, product_id, tenant_id, supplier_sku, unit_cost_amount_minor)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (supplier_id, product_id) DO UPDATE
             SET supplier_sku = EXCLUDED.supplier_sku, unit_cost_amount_minor = EXCLUDED.unit_cost_amount_minor",
        )
        .bind(supplier_id)
        .bind(product.product_id)
        .bind(tenant_id)
        .bind(clean(&product.supplier_sku))
        .bind(product.unit_cost_amount_minor)
        .execute(&mut *tx)
        .await?;
    }
    let products = products_of(&mut tx, supplier_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "supplier", supplier_id, "supplier.products", serde_json::to_value(&products).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier products saved.", products)));
}

/// `POST /api/v1/suppliers/:id/payments`. Lowers what is owed; paying more than the balance leaves the supplier in credit.
pub async fn record_payment(
    Path(supplier_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<RecordSupplierPaymentRequest>,
) -> Result<Json<ApiEnvelope<SupplierPayment>>, ApiError> {
    assert_manage(&mut tx).await?;
    if body.amount_minor <= 0 {
        return Err(ApiError::bad_request_with_code("amount_minor must be positive", "VALIDATION_FAILED"));
    }
    let method = required_text("method", &body.method, 50)?;
    get_supplier(&mut tx, supplier_id).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let payment = sqlx::query_as::<_, SupplierPayment>(
        "INSERT INTO atlas.supplier_payments (id, tenant_id, supplier_id, amount_minor, method, reference, paid_on, recorded_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, supplier_id, amount_minor, method, reference, paid_on, recorded_by_user_id, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(supplier_id)
    .bind(body.amount_minor)
    .bind(&method)
    .bind(clean(&body.reference))
    .bind(body.paid_on)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE atlas.suppliers SET balance_amount_minor = balance_amount_minor - $2 WHERE id = $1")
        .bind(supplier_id)
        .bind(body.amount_minor)
        .execute(&mut *tx)
        .await?;
    audit::record(&mut tx, tenant_id, user_id, "supplier_payment", payment.id, "supplier.payment", serde_json::to_value(&payment).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment recorded.", payment)));
}

/// `GET /api/v1/suppliers/:id/transactions`. Requires `purchase.create` or `supplier.manage`. Only this supplier's records, newest first.
pub async fn list_transactions(Path(supplier_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<SupplierStatement>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if !roles::service::has_permission(&mut tx, user_id, tenant_id, None, "supplier.manage").await? {
        let scope = roles::service::stores_with_permission(&mut tx, user_id, tenant_id, "purchase.create").await?;
        if scope == roles::service::StoreScope::Stores(Vec::new()) {
            return Err(ApiError::forbidden("Missing permission: supplier.manage"));
        }
    }
    let supplier = get_supplier(&mut tx, supplier_id).await?;
    let transactions = sqlx::query_as::<_, SupplierTransaction>(
        "SELECT * FROM (
             SELECT 'purchase_order' AS kind, po.id, po.id AS purchase_order_id, po.store_id, po.status::text AS status,
                    (COALESCE((SELECT SUM(ROUND(l.quantity_ordered * l.unit_cost_amount_minor))
                               FROM atlas.purchase_order_lines l WHERE l.purchase_order_id = po.id), 0)
                     + COALESCE((SELECT SUM(c.amount_minor) FROM atlas.purchase_order_charges c WHERE c.purchase_order_id = po.id), 0))::bigint
                        AS amount_minor,
                    0::bigint AS balance_effect_minor, po.created_at AS occurred_at
             FROM atlas.purchase_orders po WHERE po.supplier_id = $1
             UNION ALL
             SELECT 'goods_receipt', g.id, g.purchase_order_id, po.store_id, NULL,
                    (COALESCE(SUM(ROUND(gl.quantity_received * pol.unit_cost_amount_minor)), 0) + g.charges_amount_minor)::bigint,
                    (COALESCE(SUM(ROUND(gl.quantity_received * pol.unit_cost_amount_minor)), 0) + g.charges_amount_minor)::bigint,
                    g.received_at
             FROM atlas.goods_received_notes g
             JOIN atlas.purchase_orders po ON po.id = g.purchase_order_id
             LEFT JOIN atlas.goods_received_note_lines gl ON gl.goods_received_note_id = g.id
             LEFT JOIN atlas.purchase_order_lines pol ON pol.id = gl.purchase_order_line_id
             WHERE po.supplier_id = $1
             GROUP BY g.id, g.purchase_order_id, po.store_id, g.received_at, g.charges_amount_minor
             UNION ALL
             SELECT 'purchase_return', r.id, r.purchase_order_id, r.store_id, r.status::text,
                    COALESCE(SUM(ROUND(rl.quantity * rl.unit_cost_amount_minor)), 0)::bigint,
                    CASE WHEN r.status = 'confirmed' THEN -COALESCE(SUM(ROUND(rl.quantity * rl.unit_cost_amount_minor)), 0) ELSE 0 END::bigint,
                    r.created_at
             FROM atlas.purchase_returns r
             LEFT JOIN atlas.purchase_return_lines rl ON rl.purchase_return_id = r.id
             WHERE r.supplier_id = $1
             GROUP BY r.id, r.purchase_order_id, r.store_id, r.status, r.created_at
             UNION ALL
             SELECT 'payment', p.id, NULL, NULL, NULL, p.amount_minor, -p.amount_minor, p.created_at
             FROM atlas.supplier_payments p WHERE p.supplier_id = $1
         ) history
         ORDER BY occurred_at DESC, id DESC",
    )
    .bind(supplier_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(
        "Supplier transactions fetched.",
        SupplierStatement { supplier_id, balance_amount_minor: supplier.balance_amount_minor, transactions },
    )));
}
