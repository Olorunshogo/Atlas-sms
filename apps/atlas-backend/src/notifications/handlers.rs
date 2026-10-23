use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;

use super::models::{ListNotificationsQuery, MarkedRead, Notification, UnreadCount};
use super::service;

/// `GET /api/v1/notifications?status=&limit=`. Only the caller's own inbox.
pub async fn list_notifications(
    mut tx: TenantTx,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<ApiEnvelope<Vec<Notification>>>, ApiError> {
    let user_id = tx.user_id;
    let notifications = service::list(&mut tx, user_id, &query).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Notifications fetched.", notifications)));
}

pub async fn unread_count(mut tx: TenantTx) -> Result<Json<ApiEnvelope<UnreadCount>>, ApiError> {
    let user_id = tx.user_id;
    let unread = service::unread_count(&mut tx, user_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Unread count fetched.", UnreadCount { unread })));
}

pub async fn mark_read(Path(notification_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Notification>>, ApiError> {
    let user_id = tx.user_id;
    let notification = service::mark_read(&mut tx, user_id, notification_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Notification marked read.", notification)));
}

pub async fn mark_all_read(mut tx: TenantTx) -> Result<Json<ApiEnvelope<MarkedRead>>, ApiError> {
    let user_id = tx.user_id;
    let updated = service::mark_all_read(&mut tx, user_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Notifications marked read.", MarkedRead { updated })));
}
