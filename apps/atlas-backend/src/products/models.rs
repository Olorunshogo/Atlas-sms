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
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub product_type_id: Uuid,
    pub base_unit_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    #[serde(default = "default_attributes")]
    pub attributes: Value,
}

fn default_attributes() -> Value {
    return Value::Object(Default::default());
}
