use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{CreateProductRequest, CreateProductTypeRequest, Product, ProductType};

// === Product types

pub async fn list_product_types(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Vec<ProductType>, ApiError> {
    sqlx::query_as::<_, ProductType>(
        "SELECT id, tenant_id, name, attribute_schema, identity_mode::text AS identity_mode
         FROM atlas.product_types
         WHERE tenant_id = $1 AND deleted_at IS NULL
         ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_product_type(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateProductTypeRequest,
) -> Result<ProductType, ApiError> {
    sqlx::query_as::<_, ProductType>(
        "INSERT INTO atlas.product_types (id, tenant_id, name, attribute_schema, identity_mode)
         VALUES ($1, $2, $3, $4, $5::atlas.stock_identity_mode)
         RETURNING id, tenant_id, name, attribute_schema, identity_mode::text AS identity_mode",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&payload.name)
    .bind(&payload.attribute_schema)
    .bind(&payload.identity_mode)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

// === Products

pub async fn list_products(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Vec<Product>, ApiError> {
    sqlx::query_as::<_, Product>(
        "SELECT id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active
         FROM atlas.products
         WHERE tenant_id = $1 AND deleted_at IS NULL
         ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_product(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateProductRequest,
) -> Result<Product, ApiError> {
    sqlx::query_as::<_, Product>(
        "INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name, sku, attributes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.product_type_id)
    .bind(payload.base_unit_id)
    .bind(&payload.name)
    .bind(&payload.sku)
    .bind(&payload.attributes)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}
