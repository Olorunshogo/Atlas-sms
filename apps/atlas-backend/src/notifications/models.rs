use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "notification_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    LowStock,
    TransferRequested,
    TransferApproved,
    TransferRejected,
    TransferDispatched,
    TransferReceived,
    StockCountSubmitted,
    PurchaseOrderSubmitted,
    ExpenseSubmitted,
    ExpenseApproved,
    ExpenseRejected,
}

#[derive(Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "notification_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Unread,
    Read,
}

/// One inbox entry: the event plus this caller's read state.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub store_id: Option<Uuid>,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub status: NotificationStatus,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub status: Option<NotificationStatus>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct UnreadCount {
    pub unread: i64,
}

#[derive(Debug, Serialize)]
pub struct MarkedRead {
    pub updated: u64,
}

/// Who an event reaches: holders of a permission at a store, or one named user.
pub enum Audience {
    Permission { permission: &'static str, store_id: Uuid },
    User(Uuid),
}

pub struct Event {
    pub tenant_id: Uuid,
    pub store_id: Option<Uuid>,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub entity_type: &'static str,
    pub entity_id: Uuid,
    pub dedupe_key: Option<String>,
    pub audience: Vec<Audience>,
    /// The user whose own action caused the event; they aren't told about it.
    pub exclude_user_id: Option<Uuid>,
}
