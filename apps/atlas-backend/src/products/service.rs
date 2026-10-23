use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::validate::{require_visible_for, required_text};

use super::models::{CreateProductRequest, CreateProductTypeRequest, Product, ProductType};

// === Product types

pub async fn list_product_types(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Vec<ProductType>, ApiError> {
    return sqlx::query_as::<_, ProductType>(
        "SELECT id, tenant_id, name, attribute_schema, identity_mode::text AS identity_mode
         FROM atlas.product_types
         WHERE tenant_id = $1 AND deleted_at IS NULL
         ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_product_type(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateProductTypeRequest,
) -> Result<ProductType, ApiError> {
    let name = required_text("name", &payload.name, 100)?;
    if !["by_product", "by_batch", "by_serial", "by_both"].contains(&payload.identity_mode.as_str()) {
        return Err(ApiError::bad_request_with_code(
            "identity_mode must be by_product, by_batch, by_serial or by_both",
            "VALIDATION_FAILED",
        ));
    }
    if !payload.attribute_schema.is_array() {
        return Err(ApiError::bad_request_with_code("attribute_schema must be an array", "VALIDATION_FAILED"));
    }
    return sqlx::query_as::<_, ProductType>(
        "INSERT INTO atlas.product_types (id, tenant_id, name, attribute_schema, identity_mode)
         VALUES ($1, $2, $3, $4, $5::atlas.stock_identity_mode)
         RETURNING id, tenant_id, name, attribute_schema, identity_mode::text AS identity_mode",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&name)
    .bind(&payload.attribute_schema)
    .bind(&payload.identity_mode)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

// === Products

pub async fn list_products(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Vec<Product>, ApiError> {
    return sqlx::query_as::<_, Product>(
        "SELECT id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active, tax_rate_id, is_returnable
         FROM atlas.products
         WHERE tenant_id = $1 AND deleted_at IS NULL
         ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_product(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateProductRequest,
) -> Result<Product, ApiError> {
    let name = required_text("name", &payload.name, 200)?;
    let sku = match payload.sku.as_deref() {
        Some(sku) => Some(required_text("sku", sku, 64)?),
        None => None,
    };
    if !payload.attributes.is_object() {
        return Err(ApiError::bad_request_with_code("attributes must be an object", "VALIDATION_FAILED"));
    }
    require_visible_for(conn, tenant_id, "product_types", payload.product_type_id, "Product type").await?;
    require_visible_for(conn, tenant_id, "units_of_measure", payload.base_unit_id, "Unit").await?;
    return sqlx::query_as::<_, Product>(
        "INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name, sku, attributes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active, tax_rate_id, is_returnable",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.product_type_id)
    .bind(payload.base_unit_id)
    .bind(&name)
    .bind(&sku)
    .bind(&payload.attributes)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}
