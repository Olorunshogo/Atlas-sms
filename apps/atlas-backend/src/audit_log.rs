/*!
 * FR-27 audit trail search. Needs `audit.view` (owner and admin). Rows are append-only:
 * the app role can't update or delete them, and a trigger refuses any update.
 */
use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub actor_user_id: Option<Uuid>,
    /// Exact action, or a prefix ending in `.` such as `stock.` for every stock action.
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub store_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// Page backwards: only entries older than this one.
    pub before_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub device_id: Option<String>,
    pub store_id: Option<Uuid>,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub outcome: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    return Router::new().route("/", get(search));
}

/// `GET /api/v1/audit-events?actor_user_id=&action=&entity_type=&entity_id=&store_id=&from=&to=&before_id=&limit=`. Newest first.
pub async fn search(mut tx: TenantTx, Query(query): Query<AuditQuery>) -> Result<Json<ApiEnvelope<Vec<AuditEntry>>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "audit.view").await?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let (action_exact, action_prefix) = match query.action.as_deref().map(str::trim).filter(|a| !a.is_empty()) {
        Some(action) if action.ends_with('.') => (None, Some(action.to_string())),
        Some(action) => (Some(action.to_string()), None),
        None => (None, None),
    };
    let entries = sqlx::query_as::<_, AuditEntry>(
        "SELECT a.id, a.actor_user_id, u.full_name AS actor_name, a.device_id, a.store_id, a.entity_type, a.entity_id,
                a.action, a.outcome, a.before, a.after, a.created_at
         FROM atlas.audit_events a
         LEFT JOIN atlas.users u ON u.id = a.actor_user_id
         WHERE a.tenant_id = $1
           AND ($2::uuid IS NULL OR a.actor_user_id = $2)
           AND ($3::text IS NULL OR a.action = $3)
           AND ($4::text IS NULL OR a.action LIKE $4 || '%')
           AND ($5::text IS NULL OR a.entity_type = $5)
           AND ($6::uuid IS NULL OR a.entity_id = $6)
           AND ($7::uuid IS NULL OR a.store_id = $7)
           AND ($8::timestamptz IS NULL OR a.created_at >= $8)
           AND ($9::timestamptz IS NULL OR a.created_at <= $9)
           AND ($10::uuid IS NULL OR a.id < $10)
         ORDER BY a.created_at DESC, a.id DESC
         LIMIT $11",
    )
    .bind(tenant_id)
    .bind(query.actor_user_id)
    .bind(action_exact)
    .bind(action_prefix)
    .bind(query.entity_type.as_deref())
    .bind(query.entity_id)
    .bind(query.store_id)
    .bind(query.from)
    .bind(query.to)
    .bind(query.before_id)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Audit events fetched.", entries)));
}
