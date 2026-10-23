use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

/**
 * The one write path for `atlas.audit_events` - every mutation that should leave a record calls this rather than inserting directly, so the row shape can't drift between call sites.
 * `before` is `None` in this first pass (no handler fetches prior state today); `after` is the created/updated row, or `None` for actions with nothing meaningful to snapshot (e.g. a delete).
 * Runs inside the caller's own transaction, not a side connection - unlike the OTP attempt counter, an audit row for a mutation that itself rolled back should not persist.
 */
pub async fn record(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    after: Option<Value>,
) -> Result<(), ApiError> {
    return record_change(
        conn,
        Change {
            tenant_id,
            actor_user_id,
            device_id: None,
            entity_type,
            entity_id,
            action,
            before: None,
            after,
        },
    )
    .await;
}

pub struct Change<'a> {
    pub tenant_id: Uuid,
    pub actor_user_id: Uuid,
    pub device_id: Option<&'a str>,
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    pub action: &'a str,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// The full form of `record`, for edits where the prior state matters (customer updates, price drift on a synced sale) and for actions tied to a specific till.
pub async fn record_change(conn: &mut PgConnection, change: Change<'_>) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO atlas.audit_events
            (id, tenant_id, actor_user_id, device_id, entity_type, entity_id, action, before, after)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::now_v7())
    .bind(change.tenant_id)
    .bind(change.actor_user_id)
    .bind(change.device_id)
    .bind(change.entity_type)
    .bind(change.entity_id)
    .bind(change.action)
    .bind(change.before)
    .bind(change.after)
    .execute(conn)
    .await?;
    return Ok(());
}
