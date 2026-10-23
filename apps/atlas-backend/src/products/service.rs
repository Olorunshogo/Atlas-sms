use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::validate::{require_visible_for, required_text};

use super::categories::assert_assignable;
use super::models::{CreateProductRequest, CreateProductTypeRequest, ListProductsQuery, Product, ProductStatusFilter, ProductType, PRODUCT_COLUMNS};

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
    query: &ListProductsQuery,
) -> Result<Vec<Product>, ApiError> {
    let search = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    return sqlx::query_as::<_, Product>(&format!(
        "SELECT {PRODUCT_COLUMNS}
         FROM atlas.products p
         WHERE tenant_id = $1 AND deleted_at IS NULL
           AND ($2::text IS NULL
                OR name ILIKE '%' || $2 || '%'
                OR sku ILIKE '%' || $2 || '%'
                OR EXISTS (SELECT 1 FROM atlas.product_barcodes b WHERE b.product_id = p.id AND b.barcode = $2))
           AND ($3::uuid IS NULL OR category_id = $3)
           AND ($4::uuid IS NULL OR product_type_id = $4)
           AND ($5::uuid IS NULL OR parent_product_id = $5)
           AND ($6::boolean IS NULL OR is_active = $6)
         ORDER BY name"
    ))
    .bind(tenant_id)
    .bind(search)
    .bind(query.category_id)
    .bind(query.product_type_id)
    .bind(query.parent_product_id)
    .bind(query.status.map(|status| status == ProductStatusFilter::Active))
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
    if let Some(category_id) = payload.category_id {
        assert_assignable(conn, tenant_id, category_id).await?;
    }
    if let Some(parent_id) = payload.parent_product_id {
        assert_variant_parent(conn, tenant_id, parent_id).await?;
    }
    return sqlx::query_as::<_, Product>(&format!(
        "INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, category_id, parent_product_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING {PRODUCT_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.product_type_id)
    .bind(payload.base_unit_id)
    .bind(&name)
    .bind(&sku)
    .bind(&payload.attributes)
    .bind(payload.category_id)
    .bind(payload.parent_product_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

/// Variants are one level deep: the parent must exist and not itself be a variant.
pub async fn assert_variant_parent(conn: &mut PgConnection, tenant_id: Uuid, parent_id: Uuid) -> Result<(), ApiError> {
    let grandparent: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT parent_product_id FROM atlas.products WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
    )
    .bind(parent_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?;
    return match grandparent {
        None => Err(ApiError::not_found("Parent product not found.")),
        Some(Some(_)) => Err(ApiError::bad_request_with_code("A variant cannot have variants of its own.", "VALIDATION_FAILED")),
        Some(None) => Ok(()),
    };
}
