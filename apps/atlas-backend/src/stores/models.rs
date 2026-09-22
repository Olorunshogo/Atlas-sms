use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Store {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub currency: String,
    pub is_active: bool,
}
