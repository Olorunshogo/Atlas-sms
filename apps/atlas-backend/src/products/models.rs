use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::validate::double_option;

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
    Value::Array(vec![])
}

fn default_identity_mode() -> String {
    "by_product".to_string()
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
    Value::Object(Default::default())
}

/**
 * Every field optional; absent means unchanged, `sku: null` clears the SKU.
 * Product type and base unit are not editable (stock identity and every
 * quantity are defined in terms of them), and unknown fields are rejected
 * so a client trying to change them gets an error instead of silence.
 */
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub sku: Option<Option<String>>,
    pub attributes: Option<Value>,
    pub is_active: Option<bool>,
}

impl UpdateProductRequest {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.sku.is_none()
            && self.attributes.is_none()
            && self.is_active.is_none()
    }
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleUnit {
    pub unit_id: Uuid,
    pub unit_name: String,
    pub factor_to_base: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateSaleUnitRequest {
    pub unit_id: Uuid,
    /// How many base units one of this unit is (a carton of 12 pieces is 12).
    pub factor_to_base: Decimal,
}
