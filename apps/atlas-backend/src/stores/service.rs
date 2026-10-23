use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::validate::{nothing_to_update, required_text, slug};

use super::models::{CreateStoreRequest, Store, UpdateStoreRequest};

const COLUMNS: &str = "id, tenant_id, name, slug, currency, address, is_active";

pub async fn list_stores(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<Store>, ApiError> {
    sqlx::query_as::<_, Store>(&format!(
        "SELECT {COLUMNS} FROM atlas.stores
             WHERE tenant_id = $1 AND deleted_at IS NULL
             ORDER BY created_at"
    ))
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn get_store(conn: &mut PgConnection, store_id: Uuid) -> Result<Store, ApiError> {
    sqlx::query_as::<_, Store>(&format!(
        "SELECT {COLUMNS} FROM atlas.stores WHERE id = $1 AND deleted_at IS NULL"
    ))
    .bind(store_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Store not found.".to_string()))
}

/** Blank address is stored as no address. */
fn clean_address(address: &str) -> Result<Option<String>, ApiError> {
    if address.trim().is_empty() {
        return Ok(None);
    }
    required_text("address", address, 255).map(Some)
}

pub async fn create_store(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateStoreRequest,
) -> Result<Store, ApiError> {
    let name = required_text("name", &payload.name, 100)?;
    let slug = slug("slug", &payload.slug)?;
    let address = match payload.address.as_deref() {
        Some(address) => clean_address(address)?,
        None => None,
    };
    sqlx::query_as::<_, Store>(&format!(
        "INSERT INTO atlas.stores (id, tenant_id, name, slug, address)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .bind(slug)
    .bind(address)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

/**
 * Partial update. Currency and timezone are deliberately not editable:
 * every price rule and sale at the store is in that currency's minor units.
 * A store cannot be deactivated while a shift is open, so a till is never
 * left mid-shift with nothing allowed to sell.
 */
pub async fn update_store(
    conn: &mut PgConnection,
    store_id: Uuid,
    payload: &UpdateStoreRequest,
) -> Result<Store, ApiError> {
    if payload.is_empty() {
        return Err(nothing_to_update());
    }
    let name = match payload.name.as_deref() {
        Some(name) => Some(required_text("name", name, 100)?),
        None => None,
    };
    let slug = match payload.slug.as_deref() {
        Some(value) => Some(slug("slug", value)?),
        None => None,
    };
    let (set_address, address) = match &payload.address {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(address)) => (true, clean_address(address)?),
    };

    if payload.is_active == Some(false) {
        let open_shift: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM atlas.shifts WHERE store_id = $1 AND status = 'open' LIMIT 1",
        )
        .bind(store_id)
        .fetch_optional(&mut *conn)
        .await?;
        if open_shift.is_some() {
            return Err(ApiError::conflict_with_code(
                "Close the open shift before deactivating this store.",
                "SHIFT_OPEN",
            ));
        }
    }

    sqlx::query_as::<_, Store>(&format!(
        "UPDATE atlas.stores
         SET name = COALESCE($2, name),
             slug = COALESCE($3, slug),
             address = CASE WHEN $4 THEN $5 ELSE address END,
             is_active = COALESCE($6, is_active),
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {COLUMNS}"
    ))
    .bind(store_id)
    .bind(name)
    .bind(slug)
    .bind(set_address)
    .bind(address)
    .bind(payload.is_active)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Store not found.".to_string()))
}
