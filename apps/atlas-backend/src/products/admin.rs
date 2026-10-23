/*!
 * Catalogue administration that the selling setup in `selling.rs` assumes already exists: the units of measure a product is counted and sold in, reading a single product, and listing or removing sale units and price rules.
 * Writes are gated on `product.manage`; reads are open to any member of the tenant, like the product list.
 */
use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{require_visible, required_text};

use super::models::{Product, PRODUCT_COLUMNS};
use super::selling::{PriceRule, SaleUnit};

async fn assert_manage(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await;
}

// === Units

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Unit {
    pub id: Uuid,
    pub name: String,
    pub abbreviation: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUnitRequest {
    pub name: String,
    pub abbreviation: String,
}

/// `GET /api/v1/units`. Tenant-wide, like products.
pub async fn list_units(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Unit>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let units = sqlx::query_as::<_, Unit>("SELECT id, name, abbreviation FROM atlas.units_of_measure WHERE tenant_id = $1 ORDER BY name")
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Units fetched.", units)));
}

/// `POST /api/v1/units`. Requires `product.manage`. A product's base unit and its sale units are rows here (piece, kg, carton).
pub async fn create_unit(mut tx: TenantTx, Json(body): Json<CreateUnitRequest>) -> Result<Json<ApiEnvelope<Unit>>, ApiError> {
    assert_manage(&mut tx).await?;
    let name = required_text("name", &body.name, 50)?;
    let abbreviation = required_text("abbreviation", &body.abbreviation, 10)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let unit = sqlx::query_as::<_, Unit>(
        "INSERT INTO atlas.units_of_measure (id, tenant_id, name, abbreviation) VALUES ($1, $2, $3, $4) RETURNING id, name, abbreviation",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .bind(abbreviation)
    .fetch_one(&mut *tx)
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "unit", unit.id, "unit.create", serde_json::to_value(&unit).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Unit created.", unit)));
}

// === Single product

/// `GET /api/v1/products/:id`.
pub async fn get_product(Path(product_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let product = sqlx::query_as::<_, Product>(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM atlas.products WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"
    ))
    .bind(product_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Product not found."))?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product fetched.", product)));
}

// === Sale units and price rules

/// `GET /api/v1/products/:id/sale-units`.
pub async fn list_sale_units(Path(product_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<SaleUnit>>>, ApiError> {
    require_visible(&mut tx, "products", product_id, "Product").await?;
    let units = sqlx::query_as::<_, SaleUnit>(
        "SELECT psu.id, psu.product_id, psu.unit_id, psu.factor_to_base
         FROM atlas.product_sale_units psu JOIN atlas.units_of_measure u ON u.id = psu.unit_id
         WHERE psu.product_id = $1 ORDER BY u.name",
    )
    .bind(product_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale units fetched.", units)));
}

/// `GET /api/v1/products/:id/price-rules`. Requires `product.manage`: prices are a catalogue decision, not something every till needs to read as rules.
pub async fn list_price_rules(Path(product_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<PriceRule>>>, ApiError> {
    assert_manage(&mut tx).await?;
    require_visible(&mut tx, "products", product_id, "Product").await?;
    let rules = sqlx::query_as::<_, PriceRule>(
        "SELECT id, product_id, store_id, customer_group_id, min_quantity, unit_price_amount_minor, priority, starts_at, ends_at
         FROM atlas.price_rules WHERE product_id = $1 ORDER BY created_at, id",
    )
    .bind(product_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Price rules fetched.", rules)));
}

/// `DELETE /api/v1/products/:id/price-rules/:rule_id`. Requires `product.manage`. Safe to hard-delete: sale lines copy the price they sold at.
pub async fn delete_price_rule(Path((product_id, rule_id)): Path<(Uuid, Uuid)>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    assert_manage(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let rule = sqlx::query_as::<_, PriceRule>(
        "DELETE FROM atlas.price_rules WHERE id = $1 AND product_id = $2 AND tenant_id = $3
         RETURNING id, product_id, store_id, customer_group_id, min_quantity, unit_price_amount_minor, priority, starts_at, ends_at",
    )
    .bind(rule_id)
    .bind(product_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Price rule not found."))?;
    super::price_history::record_rule_deleted(&mut tx, tenant_id, user_id, &rule).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "price_rule",
            entity_id: rule_id,
            action: "price_rule.delete",
            before: serde_json::to_value(&rule).ok(),
            after: None,
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Price rule deleted.", ())));
}
