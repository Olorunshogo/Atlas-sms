use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::validate::{nothing_to_update, required_text};

use super::models::{
    CreateProductRequest, CreateProductTypeRequest, CreateSaleUnitRequest, CreateUnitRequest,
    Product, ProductType, SaleUnit, Unit, UpdateProductRequest,
};

const IDENTITY_MODES: [&str; 4] = ["by_product", "by_batch", "by_serial", "by_both"];

/**
 * Foreign keys do not go through RLS, so a tenant could otherwise point a
 * new record at another tenant's row. Reading through the scoped
 * transaction turns a foreign id into "not found".
 */
async fn require_visible(
    conn: &mut PgConnection,
    table: &str,
    id: Uuid,
    label: &str,
) -> Result<(), ApiError> {
    let found: Option<i32> =
        sqlx::query_scalar(&format!("SELECT 1 FROM atlas.{table} WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?;
    found
        .map(|_| ())
        .ok_or_else(|| ApiError::NotFound(format!("{label} not found.")))
}

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
    let name = required_text("name", &payload.name, 100)?;
    if !IDENTITY_MODES.contains(&payload.identity_mode.as_str()) {
        return Err(ApiError::bad_request_with_code(
            "identity_mode must be by_product, by_batch, by_serial or by_both",
            "VALIDATION_FAILED",
        ));
    }
    if !payload.attribute_schema.is_array() {
        return Err(ApiError::bad_request_with_code(
            "attribute_schema must be an array",
            "VALIDATION_FAILED",
        ));
    }
    sqlx::query_as::<_, ProductType>(
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
    let name = required_text("name", &payload.name, 200)?;
    let sku = match payload.sku.as_deref() {
        None => None,
        Some(sku) => Some(required_text("sku", sku, 64)?),
    };
    if !payload.attributes.is_object() {
        return Err(ApiError::bad_request_with_code(
            "attributes must be an object",
            "VALIDATION_FAILED",
        ));
    }
    require_visible(
        conn,
        "product_types",
        payload.product_type_id,
        "Product type",
    )
    .await?;
    require_visible(conn, "units_of_measure", payload.base_unit_id, "Unit").await?;
    sqlx::query_as::<_, Product>(
        "INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name, sku, attributes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active",
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
    .map_err(ApiError::from)
}

pub async fn get_product(conn: &mut PgConnection, product_id: Uuid) -> Result<Product, ApiError> {
    sqlx::query_as::<_, Product>(
        "SELECT id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active
         FROM atlas.products WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(product_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Product not found.".to_string()))
}

/**
 * Partial update. Deactivating a product stops it being sold, restocked or
 * transferred but keeps its stock, ledger and sale history intact; it is
 * the supported alternative to deleting one.
 */
pub async fn update_product(
    conn: &mut PgConnection,
    product_id: Uuid,
    payload: &UpdateProductRequest,
) -> Result<Product, ApiError> {
    if payload.is_empty() {
        return Err(nothing_to_update());
    }
    let name = match payload.name.as_deref() {
        Some(name) => Some(required_text("name", name, 200)?),
        None => None,
    };
    let (set_sku, sku) = match &payload.sku {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(sku)) => (true, Some(required_text("sku", sku, 64)?)),
    };
    if let Some(attributes) = &payload.attributes {
        if !attributes.is_object() {
            return Err(ApiError::bad_request_with_code(
                "attributes must be an object",
                "VALIDATION_FAILED",
            ));
        }
    }

    sqlx::query_as::<_, Product>(
        "UPDATE atlas.products
         SET name = COALESCE($2, name),
             sku = CASE WHEN $3 THEN $4 ELSE sku END,
             attributes = COALESCE($5, attributes),
             is_active = COALESCE($6, is_active),
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active",
    )
    .bind(product_id)
    .bind(name)
    .bind(set_sku)
    .bind(sku)
    .bind(&payload.attributes)
    .bind(payload.is_active)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Product not found.".to_string()))
}

// === Units

pub async fn list_units(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<Unit>, ApiError> {
    sqlx::query_as::<_, Unit>(
        "SELECT id, name, abbreviation FROM atlas.units_of_measure WHERE tenant_id = $1 ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_unit(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateUnitRequest,
) -> Result<Unit, ApiError> {
    let name = required_text("name", &payload.name, 50)?;
    let abbreviation = required_text("abbreviation", &payload.abbreviation, 10)?;
    sqlx::query_as::<_, Unit>(
        "INSERT INTO atlas.units_of_measure (id, tenant_id, name, abbreviation)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, abbreviation",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .bind(abbreviation)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

// === Sale units

pub async fn list_sale_units(
    conn: &mut PgConnection,
    product_id: Uuid,
) -> Result<Vec<SaleUnit>, ApiError> {
    require_visible(conn, "products", product_id, "Product").await?;
    sqlx::query_as::<_, SaleUnit>(
        "SELECT psu.unit_id, u.name AS unit_name, psu.factor_to_base
         FROM atlas.product_sale_units psu
         JOIN atlas.units_of_measure u ON u.id = psu.unit_id
         WHERE psu.product_id = $1
         ORDER BY u.name",
    )
    .bind(product_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

/**
 * Declares an alternate unit a product may be sold in. The base unit is
 * implicit (factor 1) and cannot be re-declared; a repeat declaration of
 * the same unit is a 409 from the (product, unit) unique constraint.
 */
pub async fn create_sale_unit(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    product_id: Uuid,
    payload: &CreateSaleUnitRequest,
) -> Result<SaleUnit, ApiError> {
    if payload.factor_to_base <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::bad_request_with_code(
            "factor_to_base must be greater than zero",
            "VALIDATION_FAILED",
        ));
    }
    let base_unit_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT base_unit_id FROM atlas.products WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(product_id)
    .fetch_optional(&mut *conn)
    .await?;
    let base_unit_id =
        base_unit_id.ok_or_else(|| ApiError::NotFound("Product not found.".to_string()))?;
    require_visible(conn, "units_of_measure", payload.unit_id, "Unit").await?;
    if payload.unit_id == base_unit_id {
        return Err(ApiError::bad_request_with_code(
            "the base unit is already sellable; declare a different unit",
            "VALIDATION_FAILED",
        ));
    }
    sqlx::query_as::<_, SaleUnit>(
        "WITH inserted AS (
            INSERT INTO atlas.product_sale_units (id, tenant_id, product_id, unit_id, factor_to_base)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING unit_id, factor_to_base
         )
         SELECT i.unit_id, u.name AS unit_name, i.factor_to_base
         FROM inserted i JOIN atlas.units_of_measure u ON u.id = i.unit_id",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(product_id)
    .bind(payload.unit_id)
    .bind(payload.factor_to_base)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}
