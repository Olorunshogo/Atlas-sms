use axum::Json;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{CreateProductRequest, CreateProductTypeRequest, Product, ProductType};
use super::service;

/**
 * `GET /api/v1/product-types`.
 * Tenant-wide, same visibility rule as stores.
 */
pub async fn list_product_types(
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<ProductType>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let types = service::list_product_types(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product types fetched.", types)));
}

/**
 * `POST /api/v1/product-types`.
 * Requires `product.manage` - defining a new product type (with its attribute schema and stock-identity mode) is a catalogue decision, not a per-sale one.
 */
pub async fn create_product_type(
    mut tx: TenantTx,
    Json(body): Json<CreateProductTypeRequest>,
) -> Result<Json<ApiEnvelope<ProductType>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let product_type = service::create_product_type(&mut tx, tenant_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "product_type",
        product_type.id,
        "product_type.create",
        serde_json::to_value(&product_type).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product type created.", product_type)));
}

/**
 * `GET /api/v1/products`.
 * Tenant-wide visibility; per-store presence and cost live in `product_store_listings`, not filtered here.
 */
pub async fn list_products(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Product>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let products = service::list_products(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Products fetched.", products)));
}

/**
 * `POST /api/v1/products`.
 * Requires `product.manage`.
 */
pub async fn create_product(
    mut tx: TenantTx,
    Json(body): Json<CreateProductRequest>,
) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let product = service::create_product(&mut tx, tenant_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "product",
        product.id,
        "product.create",
        serde_json::to_value(&product).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product created.", product)));
}
