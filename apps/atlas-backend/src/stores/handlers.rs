use axum::Json;
use serde::Deserialize;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::Store;
use super::service;

#[derive(Deserialize)]
pub struct CreateStoreRequest {
    pub name: String,
    pub slug: String,
}

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
    let store = service::create_store(&mut tx, tenant_id, &body.name, &body.slug).await?;
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
