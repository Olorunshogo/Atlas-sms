use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CreateProductRequest, CreateProductTypeRequest, CreateSaleUnitRequest, CreateUnitRequest,
    Product, ProductType, SaleUnit, Unit, UpdateProductRequest,
};
use super::service;

/// `GET /api/v1/product-types`. Tenant-wide, same visibility rule as stores.
pub async fn list_product_types(
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<ProductType>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let types = service::list_product_types(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Product types fetched.", types)))
}

/// `POST /api/v1/product-types`. Requires `product.manage` - defining a new
/// product type (with its attribute schema and stock-identity mode) is a
/// catalogue decision, not a per-sale one.
pub async fn create_product_type(
    mut tx: TenantTx,
    Json(body): Json<CreateProductTypeRequest>,
) -> Result<Json<ApiEnvelope<ProductType>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let product_type = service::create_product_type(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Product type created.", product_type)))
}

/// `GET /api/v1/products`. Tenant-wide visibility; per-store presence and
/// cost live in `product_store_listings`, not filtered here.
pub async fn list_products(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Product>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let products = service::list_products(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Products fetched.", products)))
}

/// `POST /api/v1/products`. Requires `product.manage`.
pub async fn create_product(
    mut tx: TenantTx,
    Json(body): Json<CreateProductRequest>,
) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let product = service::create_product(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Product created.", product)))
}

/// `GET /api/v1/products/{product_id}`.
pub async fn get_product(
    mut tx: TenantTx,
    Path(product_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    let product = service::get_product(&mut tx, product_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Product fetched.", product)))
}

/// `PATCH /api/v1/products/{product_id}`. Requires `product.manage`.
pub async fn update_product(
    mut tx: TenantTx,
    Path(product_id): Path<Uuid>,
    Json(body): Json<UpdateProductRequest>,
) -> Result<Json<ApiEnvelope<Product>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let product = service::update_product(&mut tx, product_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Product updated.", product)))
}

// === Units

/// `GET /api/v1/units`. Tenant-wide, like products.
pub async fn list_units(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Unit>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let units = service::list_units(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Units fetched.", units)))
}

/// `POST /api/v1/units`. Requires `product.manage`.
pub async fn create_unit(
    mut tx: TenantTx,
    Json(body): Json<CreateUnitRequest>,
) -> Result<Json<ApiEnvelope<Unit>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let unit = service::create_unit(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Unit created.", unit)))
}

// === Sale units

/// `GET /api/v1/products/{product_id}/sale-units`.
pub async fn list_sale_units(
    mut tx: TenantTx,
    Path(product_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Vec<SaleUnit>>>, ApiError> {
    let units = service::list_sale_units(&mut tx, product_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sale units fetched.", units)))
}

/// `POST /api/v1/products/{product_id}/sale-units`. Requires `product.manage`.
pub async fn create_sale_unit(
    mut tx: TenantTx,
    Path(product_id): Path<Uuid>,
    Json(body): Json<CreateSaleUnitRequest>,
) -> Result<Json<ApiEnvelope<SaleUnit>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let unit = service::create_sale_unit(&mut tx, tenant_id, product_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sale unit added.", unit)))
}
