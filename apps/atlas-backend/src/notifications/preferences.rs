/*!
 * FR-30 notification preferences: per user, per kind, whether it shows in the app and whether it is also emailed.
 * Without a saved preference a kind is in-app only. Each user manages their own.
 */
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;

use super::models::NotificationKind;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Preference {
    pub kind: NotificationKind,
    pub in_app: bool,
    pub email: bool,
}

#[derive(Debug, Deserialize)]
pub struct SetPreferencesRequest {
    pub preferences: Vec<Preference>,
}

async fn load(tx: &mut TenantTx) -> Result<Vec<Preference>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return sqlx::query_as::<_, Preference>(
        "SELECT k.kind, COALESCE(p.in_app, true) AS in_app, COALESCE(p.email, false) AS email
         FROM unnest(enum_range(NULL::atlas.notification_kind)) AS k(kind)
         LEFT JOIN atlas.notification_preferences p ON p.kind = k.kind AND p.user_id = $1 AND p.tenant_id = $2
         ORDER BY k.kind",
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(ApiError::from);
}

/// `GET /api/v1/notifications/preferences`. Every kind, with the caller's effective setting.
pub async fn get_preferences(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Preference>>>, ApiError> {
    let preferences = load(&mut tx).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Notification preferences fetched.", preferences)));
}

/// `PUT /api/v1/notifications/preferences`. Saves the kinds given; the rest keep their current setting.
pub async fn set_preferences(mut tx: TenantTx, Json(body): Json<SetPreferencesRequest>) -> Result<Json<ApiEnvelope<Vec<Preference>>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let before = load(&mut tx).await?;
    for preference in &body.preferences {
        sqlx::query(
            "INSERT INTO atlas.notification_preferences (user_id, tenant_id, kind, in_app, email)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, tenant_id, kind) DO UPDATE SET in_app = EXCLUDED.in_app, email = EXCLUDED.email",
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(preference.kind)
        .bind(preference.in_app)
        .bind(preference.email)
        .execute(&mut *tx)
        .await?;
    }
    let preferences = load(&mut tx).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "notification_preferences",
            entity_id: user_id,
            action: "notification_preferences.set",
            before: serde_json::to_value(&before).ok(),
            after: serde_json::to_value(&preferences).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Notification preferences saved.", preferences)));
}
