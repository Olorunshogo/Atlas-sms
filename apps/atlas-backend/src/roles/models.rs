use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub permissions: Vec<String>,
    pub is_system: bool,
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub role_id: Uuid,
    /// `None` grants tenant-wide; `Some` scopes the grant to that store.
    pub store_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum InviteResponseData {
    /// The email belonged to an existing, verified user - they were
    /// granted the role directly and can use it on their next login.
    GrantedExisting,
    /// No account exists yet - an invite email was sent.
    Invited,
}
