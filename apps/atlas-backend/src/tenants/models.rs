use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    /// Printed on receipts in place of `name` when set.
    pub legal_name: Option<String>,
    pub tax_identification_number: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBusinessDetailsRequest {
    pub legal_name: Option<String>,
    pub tax_identification_number: Option<String>,
}
