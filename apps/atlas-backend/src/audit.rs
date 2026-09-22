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
    sqlx::query(
        "INSERT INTO atlas.audit_events (id, tenant_id, actor_user_id, entity_type, entity_id, action, after)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(actor_user_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(action)
    .bind(after)
    .execute(conn)
    .await?;
    return Ok(());
}
