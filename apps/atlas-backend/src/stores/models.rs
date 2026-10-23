use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::validate::double_option;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Store {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub currency: String,
    pub address: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateStoreRequest {
    pub name: String,
    pub slug: String,
    pub address: Option<String>,
}

/**
 * Every field optional; absent means unchanged, and `address: null` clears it.
 * Unknown fields are rejected so a client sending `currency` or `tenant_id` finds out it did nothing instead of assuming it worked.
 */
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateStoreRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub address: Option<Option<String>>,
    pub is_active: Option<bool>,
}

impl UpdateStoreRequest {
    pub fn is_empty(&self) -> bool {
        return self.name.is_none() && self.slug.is_none() && self.address.is_none() && self.is_active.is_none();
    }
}
