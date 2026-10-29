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

/// The row as stored, for an edit's before-and-after values (FR-27). `table` and `key` are always literals, and only tables without secrets are snapshot.
pub async fn snapshot(conn: &mut PgConnection, table: &str, key: &str, id: Uuid) -> Result<Option<Value>, ApiError> {
    return sqlx::query_scalar(&format!("SELECT to_jsonb(t) FROM atlas.{} t WHERE t.{} = $1", table, key))
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(ApiError::from);
}

pub struct Edit<'a> {
    pub tenant_id: Uuid,
    pub actor_user_id: Uuid,
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    pub action: &'a str,
    pub table: &'a str,
    pub key: &'a str,
    /// From `snapshot`, taken before the change.
    pub before: Option<Value>,
}

/// Records an edit with the row as it was and as it now is, so the two read side by side.
pub async fn record_edit(conn: &mut PgConnection, edit: Edit<'_>) -> Result<(), ApiError> {
    let after = snapshot(conn, edit.table, edit.key, edit.entity_id).await?;
    return record_change(
        conn,
        Change {
            tenant_id: edit.tenant_id,
            actor_user_id: edit.actor_user_id,
            device_id: None,
            entity_type: edit.entity_type,
            entity_id: edit.entity_id,
            action: edit.action,
            before: edit.before,
            after,
        },
    )
    .await;
}

/// The approval that let a held change through, written onto the change's own audit row (FR-27).
#[derive(Debug, Clone, Copy)]
pub struct Approval {
    pub request_id: Uuid,
    pub approved_by_user_id: Uuid,
}

pub fn with_approval(after: Option<Value>, approval: Option<Approval>) -> Option<Value> {
    let (Some(Value::Object(mut fields)), Some(approval)) = (after.clone(), approval) else { return after };
    fields.insert("approval_request_id".to_string(), Value::from(approval.request_id.to_string()));
    fields.insert("approved_by_user_id".to_string(), Value::from(approval.approved_by_user_id.to_string()));
    return Some(Value::Object(fields));
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
    // The store is read off the snapshot, so every call site tags it without passing it (FR-27 store filter)
    sqlx::query(
        "INSERT INTO atlas.audit_events
            (id, tenant_id, actor_user_id, device_id, entity_type, entity_id, action, before, after, store_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                 COALESCE(
                     CASE WHEN $5 = 'store' THEN $6 END,
                     NULLIF($9->>'store_id', '')::uuid,
                     NULLIF($8->>'store_id', '')::uuid,
                     NULLIF($9->>'from_store_id', '')::uuid,
                     NULLIF($8->>'from_store_id', '')::uuid
                 ))",
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
