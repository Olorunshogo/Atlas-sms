/*!
 * The per-store sales configuration: tax treatment, rounding, discount and
 * stock policy. Read by anyone selling at the store, changed only with
 * `store.update` at that store.
 */
use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

const SETTINGS_COLUMNS: &str = "id, currency, timezone, prices_include_tax, default_tax_rate_id,
     cash_rounding_increment_minor, max_cashier_discount_basis_points, allow_negative_stock";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StoreSettings {
    pub id: Uuid,
    pub currency: String,
    pub timezone: String,
    pub prices_include_tax: bool,
    pub default_tax_rate_id: Option<Uuid>,
    pub cash_rounding_increment_minor: i64,
    pub max_cashier_discount_basis_points: i32,
    pub allow_negative_stock: bool,
}

fn explicit_null<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    return Option::<T>::deserialize(deserializer).map(Some);
}

/// Every field optional; only the ones sent change. `default_tax_rate_id: null` clears it.
#[derive(Debug, Deserialize)]
pub struct UpdateStoreSettingsRequest {
    pub prices_include_tax: Option<bool>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub default_tax_rate_id: Option<Option<Uuid>>,
    pub cash_rounding_increment_minor: Option<i64>,
    pub max_cashier_discount_basis_points: Option<i32>,
    pub allow_negative_stock: Option<bool>,
}

/// `GET /api/v1/stores/:id/settings`. Requires `sale.create` or `store.update` at the store.
pub async fn get_settings(Path(store_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StoreSettings>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if !roles::service::has_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await? {
        roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "store.update").await?;
    }
    let settings = fetch(&mut tx, store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Store settings fetched.", settings)));
}

/// `PATCH /api/v1/stores/:id/settings`. Requires `store.update` at the store.
pub async fn update_settings(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateStoreSettingsRequest>,
) -> Result<Json<ApiEnvelope<StoreSettings>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "store.update").await?;
    if body.cash_rounding_increment_minor.is_some_and(|v| v < 1) {
        return Err(ApiError::bad_request("cash_rounding_increment_minor must be at least 1"));
    }
    if body.max_cashier_discount_basis_points.is_some_and(|v| !(0..=10_000).contains(&v)) {
        return Err(ApiError::bad_request("max_cashier_discount_basis_points must be between 0 and 10000"));
    }
    let before = fetch(&mut tx, store_id).await?;

    sqlx::query(
        "UPDATE atlas.stores
         SET prices_include_tax = COALESCE($2, prices_include_tax),
             default_tax_rate_id = CASE WHEN $3 THEN $4 ELSE default_tax_rate_id END,
             cash_rounding_increment_minor = COALESCE($5, cash_rounding_increment_minor),
             max_cashier_discount_basis_points = COALESCE($6, max_cashier_discount_basis_points),
             allow_negative_stock = COALESCE($7, allow_negative_stock),
             updated_at = now()
         WHERE id = $1",
    )
    .bind(store_id)
    .bind(body.prices_include_tax)
    .bind(body.default_tax_rate_id.is_some())
    .bind(body.default_tax_rate_id.flatten())
    .bind(body.cash_rounding_increment_minor)
    .bind(body.max_cashier_discount_basis_points)
    .bind(body.allow_negative_stock)
    .execute(&mut *tx)
    .await?;

    let after = fetch(&mut tx, store_id).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "store",
            entity_id: store_id,
            action: "store.settings_update",
            before: serde_json::to_value(&before).ok(),
            after: serde_json::to_value(&after).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Store settings updated.", after)));
}

async fn fetch(conn: &mut sqlx::PgConnection, store_id: Uuid) -> Result<StoreSettings, ApiError> {
    return sqlx::query_as::<_, StoreSettings>(&format!(
        "SELECT {} FROM atlas.stores WHERE id = $1 AND deleted_at IS NULL",
        SETTINGS_COLUMNS
    ))
    .bind(store_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Store not found."));
}
