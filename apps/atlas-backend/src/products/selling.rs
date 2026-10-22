/*!
 * What makes a product sellable at a till: its per-store listing, extra
 * sale units, price rules, barcodes, and its tax and returns settings. All
 * gated on `product.manage`, the one catalogue permission.
 */
use axum::extract::Path;
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::{codes, ApiError};
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::Product;

/// Tells a field that was sent as `null` (clear it) apart from one that wasn't sent at all (leave it).
fn explicit_null<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    return Option::<T>::deserialize(deserializer).map(Some);
}

async fn assert_manage(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await;
}

// === Product settings

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub is_returnable: Option<bool>,
    /// `null` clears it back to the store default rate.
    #[serde(default, deserialize_with = "explicit_null")]
    pub tax_rate_id: Option<Option<Uuid>>,
}

/// `PATCH /api/v1/products/:id`.
pub async fn update_product(
    Path(product_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateProductRequest>,
) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    assert_manage(&mut tx).await?;
    let product = sqlx::query_as::<_, Product>(
        "UPDATE atlas.products
         SET name = COALESCE($2, name),
             is_active = COALESCE($3, is_active),
             is_returnable = COALESCE($4, is_returnable),
             tax_rate_id = CASE WHEN $5 THEN $6 ELSE tax_rate_id END,
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active, tax_rate_id, is_returnable",
    )
    .bind(product_id)
    .bind(&body.name)
    .bind(body.is_active)
    .bind(body.is_returnable)
    .bind(body.tax_rate_id.is_some())
    .bind(body.tax_rate_id.flatten())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Product not found."))?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    audit::record(&mut tx, tenant_id, user_id, "product", product.id, "product.update", serde_json::to_value(&product).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product updated.", product)));
}

// === Store listings

#[derive(Debug, Deserialize)]
pub struct UpsertListingRequest {
    pub cost_amount_minor: Option<i64>,
    pub low_stock_threshold: Option<Decimal>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Listing {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Uuid,
    pub cost_amount_minor: Option<i64>,
    pub low_stock_threshold: Decimal,
    pub is_active: bool,
}

/// `PUT /api/v1/products/:id/listings/:store_id` - lists the product at a store, or updates that listing.
pub async fn upsert_listing(
    Path((product_id, store_id)): Path<(Uuid, Uuid)>,
    mut tx: TenantTx,
    Json(body): Json<UpsertListingRequest>,
) -> Result<Json<ApiEnvelope<Listing>>, ApiError> {
    assert_manage(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let listing = sqlx::query_as::<_, Listing>(
        "INSERT INTO atlas.product_store_listings (id, tenant_id, product_id, store_id, cost_amount_minor, low_stock_threshold, is_active)
         VALUES ($1, $2, $3, $4, $5, COALESCE($6, 0), COALESCE($7, true))
         ON CONFLICT (product_id, store_id) DO UPDATE
         SET cost_amount_minor = COALESCE($5, atlas.product_store_listings.cost_amount_minor),
             low_stock_threshold = COALESCE($6, atlas.product_store_listings.low_stock_threshold),
             is_active = COALESCE($7, atlas.product_store_listings.is_active)
         RETURNING id, product_id, store_id, cost_amount_minor, low_stock_threshold, is_active",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(store_id)
    .bind(body.cost_amount_minor)
    .bind(body.low_stock_threshold)
    .bind(body.is_active)
    .fetch_one(&mut *tx)
    .await?;
    let user_id = tx.user_id;
    audit::record(&mut tx, tenant_id, user_id, "product_listing", listing.id, "product.list", serde_json::to_value(&listing).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product listed.", listing)));
}

// === Sale units

#[derive(Debug, Deserialize)]
pub struct CreateSaleUnitRequest {
    pub unit_id: Uuid,
    pub factor_to_base: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleUnit {
    pub id: Uuid,
    pub product_id: Uuid,
    pub unit_id: Uuid,
    pub factor_to_base: Decimal,
}

/// `POST /api/v1/products/:id/sale-units` - e.g. a 50kg bag of a product whose base unit is the kg.
pub async fn create_sale_unit(
    Path(product_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CreateSaleUnitRequest>,
) -> Result<Json<ApiEnvelope<SaleUnit>>, ApiError> {
    assert_manage(&mut tx).await?;
    if body.factor_to_base <= Decimal::ZERO {
        return Err(ApiError::bad_request("factor_to_base must be positive"));
    }
    let tenant_id = tx.tenant_id;
    let unit = sqlx::query_as::<_, SaleUnit>(
        "INSERT INTO atlas.product_sale_units (id, tenant_id, product_id, unit_id, factor_to_base)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, product_id, unit_id, factor_to_base",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(body.unit_id)
    .bind(body.factor_to_base)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale unit added.", unit)));
}

// === Price rules

#[derive(Debug, Deserialize)]
pub struct CreatePriceRuleRequest {
    pub store_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub min_quantity: Option<Decimal>,
    /// Per base unit.
    pub unit_price_amount_minor: i64,
    #[serde(default)]
    pub priority: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PriceRule {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub min_quantity: Decimal,
    pub unit_price_amount_minor: i64,
    pub priority: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

/// `POST /api/v1/products/:id/price-rules`. Branch prices, wholesale tiers and promotions are all rows here.
pub async fn create_price_rule(
    Path(product_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CreatePriceRuleRequest>,
) -> Result<Json<ApiEnvelope<PriceRule>>, ApiError> {
    assert_manage(&mut tx).await?;
    if body.unit_price_amount_minor < 0 {
        return Err(ApiError::bad_request("unit_price_amount_minor must not be negative"));
    }
    let tenant_id = tx.tenant_id;
    let rule = sqlx::query_as::<_, PriceRule>(
        "INSERT INTO atlas.price_rules
            (id, tenant_id, product_id, store_id, customer_group_id, min_quantity, unit_price_amount_minor, priority, starts_at, ends_at)
         VALUES ($1, $2, $3, $4, $5, COALESCE($6, 1), $7, $8, $9, $10)
         RETURNING id, product_id, store_id, customer_group_id, min_quantity, unit_price_amount_minor, priority, starts_at, ends_at",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(body.store_id)
    .bind(body.customer_group_id)
    .bind(body.min_quantity)
    .bind(body.unit_price_amount_minor)
    .bind(body.priority)
    .bind(body.starts_at)
    .bind(body.ends_at)
    .fetch_one(&mut *tx)
    .await?;
    let user_id = tx.user_id;
    audit::record(&mut tx, tenant_id, user_id, "price_rule", rule.id, "price_rule.create", serde_json::to_value(&rule).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Price rule created.", rule)));
}

// === Barcodes

#[derive(Debug, Deserialize)]
pub struct CreateBarcodeRequest {
    pub barcode: String,
    /// Defaults to the product's base unit.
    pub unit_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Barcode {
    pub id: Uuid,
    pub product_id: Uuid,
    pub unit_id: Uuid,
    pub barcode: String,
}

/// `POST /api/v1/products/:id/barcodes`. A barcode already used by another product in the tenant is a 409.
pub async fn create_barcode(
    Path(product_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CreateBarcodeRequest>,
) -> Result<Json<ApiEnvelope<Barcode>>, ApiError> {
    assert_manage(&mut tx).await?;
    let barcode_value = body.barcode.trim();
    if barcode_value.is_empty() {
        return Err(ApiError::bad_request("barcode must not be empty"));
    }
    let tenant_id = tx.tenant_id;
    let base_unit_id: Uuid = sqlx::query_scalar("SELECT base_unit_id FROM atlas.products WHERE id = $1 AND deleted_at IS NULL")
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("Product not found."))?;
    let unit_id = body.unit_id.unwrap_or(base_unit_id);
    crate::inventory::service::unit_factor(&mut tx, product_id, unit_id).await?;

    let barcode = sqlx::query_as::<_, Barcode>(
        "INSERT INTO atlas.product_barcodes (id, tenant_id, product_id, unit_id, barcode)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, product_id, unit_id, barcode",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(unit_id)
    .bind(barcode_value)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("That barcode is already assigned to a product.", codes::BARCODE_EXISTS)
        }
        other => ApiError::from(other),
    })?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Barcode added.", barcode)));
}
