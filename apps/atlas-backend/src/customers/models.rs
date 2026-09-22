use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub customer_group_id: Option<Uuid>,
    pub store_credit_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub customer_group_id: Option<Uuid>,
}

/// Every field optional - only the ones present are changed, the rest keep their current value (see `service::update_customer`'s `COALESCE`).
#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub customer_group_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListCustomersQuery {
    /// Matched against full_name, phone, and email - see `service::list_customers`'s doc comment for why this is `ILIKE`, not a real search index, for now.
    pub query: Option<String>,
}
