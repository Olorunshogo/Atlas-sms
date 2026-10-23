use axum::extract::Path;
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{CreateStoreRequest, Store, UpdateStoreRequest};
use super::service;

/**
 * `GET /api/v1/stores`.
 * Lists every store in the caller's tenant - visibility is tenant-wide by design; per-store role grants gate what a user can *do* at a store, not whether they can see it exists.
 */
pub async fn list_stores(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Store>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let stores = service::list_stores(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stores fetched.", stores)));
}

/**
 * `POST /api/v1/stores`.
 * Requires `store.create` - in practice only `owner`/`admin` hold it by default, but the check is generic: whoever holds the permission may open a new branch.
 * This is the "owner branches out" path - no schema change, just a new row.
 */
pub async fn create_store(
    mut tx: TenantTx,
    Json(body): Json<CreateStoreRequest>,
) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "store.create").await?;
    let store = service::create_store(&mut tx, tenant_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "store",
        store.id,
        "store.create",
        serde_json::to_value(&store).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Store created.", store)));
}

/// `GET /api/v1/stores/:id`. Tenant-wide visibility, like the list.
pub async fn get_store(Path(store_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let store = service::get_store(&mut tx, tenant_id, store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Store fetched.", store)));
}

/**
 * `PATCH /api/v1/stores/:id`.
 * Requires `store.update` at that store, so a store-scoped admin can edit their own branch and no other.
 */
pub async fn update_store(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateStoreRequest>,
) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "store.update").await?;
    let before = service::get_store(&mut tx, tenant_id, store_id).await?;
    let store = service::update_store(&mut tx, tenant_id, store_id, &body).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "store",
            entity_id: store.id,
            action: "store.update",
            before: serde_json::to_value(&before).ok(),
            after: serde_json::to_value(&store).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Store updated.", store)));
}
