use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductType {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub attribute_schema: Value,
    pub identity_mode: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductTypeRequest {
    pub name: String,
    #[serde(default = "default_attribute_schema")]
    pub attribute_schema: Value,
    #[serde(default = "default_identity_mode")]
    pub identity_mode: String,
}

fn default_attribute_schema() -> Value {
    return Value::Array(vec![]);
}

fn default_identity_mode() -> String {
    return "by_product".to_string();
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub product_type_id: Uuid,
    pub base_unit_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub attributes: Value,
    pub is_active: bool,
    pub tax_rate_id: Option<Uuid>,
    pub is_returnable: bool,
    pub category_id: Option<Uuid>,
    pub parent_product_id: Option<Uuid>,
}

pub const PRODUCT_COLUMNS: &str =
    "id, tenant_id, product_type_id, base_unit_id, name, sku, attributes, is_active, tax_rate_id, is_returnable, category_id, parent_product_id";

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub product_type_id: Uuid,
    pub base_unit_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    #[serde(default = "default_attributes")]
    pub attributes: Value,
    pub category_id: Option<Uuid>,
    /// Makes this product a variant of that one (FR-08).
    pub parent_product_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatusFilter {
    Active,
    Archived,
}

/// `q` matches name or SKU (case-insensitive substring) or an exact barcode.
#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    pub q: Option<String>,
    pub category_id: Option<Uuid>,
    pub product_type_id: Option<Uuid>,
    pub parent_product_id: Option<Uuid>,
    pub status: Option<ProductStatusFilter>,
}

fn default_attributes() -> Value {
    return Value::Object(Default::default());
}
