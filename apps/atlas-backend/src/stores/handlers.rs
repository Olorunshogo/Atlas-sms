use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{CreateStoreRequest, Store, UpdateStoreRequest};
use super::service;

/// `GET /api/v1/stores`. Lists every store in the caller's tenant -
/// visibility is tenant-wide by design; per-store role grants gate what a
/// user can *do* at a store, not whether they can see it exists.
pub async fn list(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Store>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let stores = service::list_stores(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stores fetched.", stores)))
}

/// `POST /api/v1/stores`. Requires `store.create` - in practice only
/// `owner`/`admin` hold it by default, but the check is generic: whoever
/// holds the permission may open a new branch. This is the "owner branches
/// out" path - no schema change, just a new row.
pub async fn create(
    mut tx: TenantTx,
    Json(body): Json<CreateStoreRequest>,
) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "store.create").await?;
    let store = service::create_store(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Store created.", store)))
}

/// `GET /api/v1/stores/{id}`. Tenant-wide visibility, like the list.
pub async fn get(
    mut tx: TenantTx,
    Path(store_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let store = service::get_store(&mut tx, store_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Store fetched.", store)))
}

/// `PATCH /api/v1/stores/{id}`. Requires `store.update` at that store, so a
/// store-scoped admin can edit their own branch and no other.
pub async fn update(
    mut tx: TenantTx,
    Path(store_id): Path<Uuid>,
    Json(body): Json<UpdateStoreRequest>,
) -> Result<Json<ApiEnvelope<Store>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "store.update")
        .await?;
    let store = service::update_store(&mut tx, store_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Store updated.", store)))
}
