use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::validate::double_option;

// === Customer groups

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CustomerGroup {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomerGroupRequest {
    pub name: String,
}

// === Customers

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub customer_group_id: Option<Uuid>,
    pub customer_group_name: Option<String>,
    /// Read-only here: nothing in the API moves store credit yet.
    pub store_credit_amount_minor: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// Group membership changes the prices a customer is charged, so
    /// setting it needs `product.manage`, unlike the rest of the profile.
    pub customer_group_id: Option<Uuid>,
}

/**
 * Absent leaves a field alone; `null` clears `phone`, `email` and
 * `customer_group_id`. Unknown fields (including `store_credit_amount_minor`)
 * are rejected rather than ignored.
 */
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCustomerRequest {
    pub full_name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub customer_group_id: Option<Option<Uuid>>,
}

impl UpdateCustomerRequest {
    pub fn is_empty(&self) -> bool {
        self.full_name.is_none()
            && self.phone.is_none()
            && self.email.is_none()
            && self.customer_group_id.is_none()
    }
}
