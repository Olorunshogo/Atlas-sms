/*!
 * Staff setup of a store's storefront and its shipping zones. Needs `storefront.manage` at the store.
 */
use axum::extract::Path;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{require_visible, required_text};

use super::context::{Storefront, STOREFRONT_COLUMNS};

#[derive(Debug, Deserialize)]
pub struct UpsertStorefrontRequest {
    pub handle: String,
    pub is_enabled: Option<bool>,
    pub reservation_minutes: Option<i32>,
    pub ussd_window_minutes: Option<i32>,
    pub bank_transfer_window_minutes: Option<i32>,
    pub guest_cart_days: Option<i32>,
    /// `null` turns pay on delivery off.
    pub pay_on_delivery_max_minor: Option<i64>,
    pub pay_on_delivery_requires_account: Option<bool>,
    /// Spend that earns one point; `null` turns loyalty off.
    pub loyalty_minor_per_point: Option<i64>,
    pub loyalty_point_value_minor: Option<i64>,
    pub loyalty_max_redeem_basis_points: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShippingZone {
    pub id: Uuid,
    pub store_id: Uuid,
    pub name: String,
    pub regions: Vec<String>,
    pub base_fee_minor: i64,
    pub per_kg_fee_minor: i64,
    pub allows_pay_on_delivery: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ShippingZoneRequest {
    pub name: String,
    #[serde(default)]
    pub regions: Vec<String>,
    pub base_fee_minor: i64,
    #[serde(default)]
    pub per_kg_fee_minor: i64,
    #[serde(default)]
    pub allows_pay_on_delivery: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShippingZoneRequest {
    pub name: Option<String>,
    pub regions: Option<Vec<String>>,
    pub base_fee_minor: Option<i64>,
    pub per_kg_fee_minor: Option<i64>,
    pub allows_pay_on_delivery: Option<bool>,
    pub is_active: Option<bool>,
}

pub const ZONE_COLUMNS: &str = "id, store_id, name, regions, base_fee_minor, per_kg_fee_minor, allows_pay_on_delivery, is_active, created_at";

async fn assert_manage(tx: &mut TenantTx, store_id: Uuid) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), "storefront.manage").await?;
    return require_visible(tx, "stores", store_id, "Store").await;
}

fn positive(value: Option<i32>, name: &str) -> Result<(), ApiError> {
    if value.is_some_and(|v| v <= 0) {
        return Err(ApiError::bad_request_with_code(format!("{} must be positive", name), "VALIDATION_FAILED"));
    }
    return Ok(());
}

/// `GET /api/v1/storefronts/:store_id`.
pub async fn get_storefront(Path(store_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Storefront>>, ApiError> {
    assert_manage(&mut tx, store_id).await?;
    let storefront = sqlx::query_as::<_, Storefront>(&format!("SELECT {STOREFRONT_COLUMNS} FROM atlas.storefronts WHERE store_id = $1"))
        .bind(store_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("This store has no storefront yet."))?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Storefront fetched.", storefront)));
}

/// `PUT /api/v1/storefronts/:store_id`. Creates or updates; only the fields sent change on an update.
pub async fn upsert_storefront(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpsertStorefrontRequest>,
) -> Result<Json<ApiEnvelope<Storefront>>, ApiError> {
    assert_manage(&mut tx, store_id).await?;
    let handle = required_text("handle", &body.handle, 63)?.to_lowercase();
    if !handle.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') || handle.len() < 2 || handle.starts_with('-') {
        return Err(ApiError::bad_request_with_code("handle is 2-63 lowercase letters, digits or dashes", "VALIDATION_FAILED"));
    }
    positive(body.reservation_minutes, "reservation_minutes")?;
    positive(body.ussd_window_minutes, "ussd_window_minutes")?;
    positive(body.bank_transfer_window_minutes, "bank_transfer_window_minutes")?;
    positive(body.guest_cart_days, "guest_cart_days")?;
    if body.loyalty_max_redeem_basis_points.is_some_and(|v| !(0..=10_000).contains(&v))
        || body.pay_on_delivery_max_minor.is_some_and(|v| v <= 0)
        || body.loyalty_minor_per_point.is_some_and(|v| v <= 0)
        || body.loyalty_point_value_minor.is_some_and(|v| v <= 0)
    {
        return Err(ApiError::bad_request_with_code("limits and loyalty rates must be positive", "VALIDATION_FAILED"));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let storefront = sqlx::query_as::<_, Storefront>(&format!(
        "INSERT INTO atlas.storefronts
            (store_id, tenant_id, handle, is_enabled, reservation_minutes, ussd_window_minutes, bank_transfer_window_minutes,
             guest_cart_days, pay_on_delivery_max_minor, pay_on_delivery_requires_account, loyalty_minor_per_point,
             loyalty_point_value_minor, loyalty_max_redeem_basis_points)
         VALUES ($1, $2, $3, COALESCE($4, false), COALESCE($5, 15), COALESCE($6, 30), COALESCE($7, 1440), COALESCE($8, 30),
                 $9, COALESCE($10, true), $11, COALESCE($12, 100), COALESCE($13, 2000))
         ON CONFLICT (store_id) DO UPDATE SET
             handle = EXCLUDED.handle,
             is_enabled = COALESCE($4, atlas.storefronts.is_enabled),
             reservation_minutes = COALESCE($5, atlas.storefronts.reservation_minutes),
             ussd_window_minutes = COALESCE($6, atlas.storefronts.ussd_window_minutes),
             bank_transfer_window_minutes = COALESCE($7, atlas.storefronts.bank_transfer_window_minutes),
             guest_cart_days = COALESCE($8, atlas.storefronts.guest_cart_days),
             pay_on_delivery_max_minor = $9,
             pay_on_delivery_requires_account = COALESCE($10, atlas.storefronts.pay_on_delivery_requires_account),
             loyalty_minor_per_point = $11,
             loyalty_point_value_minor = COALESCE($12, atlas.storefronts.loyalty_point_value_minor),
             loyalty_max_redeem_basis_points = COALESCE($13, atlas.storefronts.loyalty_max_redeem_basis_points),
             updated_at = now()
         RETURNING {STOREFRONT_COLUMNS}"
    ))
    .bind(store_id)
    .bind(tenant_id)
    .bind(&handle)
    .bind(body.is_enabled)
    .bind(body.reservation_minutes)
    .bind(body.ussd_window_minutes)
    .bind(body.bank_transfer_window_minutes)
    .bind(body.guest_cart_days)
    .bind(body.pay_on_delivery_max_minor)
    .bind(body.pay_on_delivery_requires_account)
    .bind(body.loyalty_minor_per_point)
    .bind(body.loyalty_point_value_minor)
    .bind(body.loyalty_max_redeem_basis_points)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::conflict_with_code("That handle is taken.", "ALREADY_EXISTS"),
        other => ApiError::from(other),
    })?;
    audit::record(&mut tx, tenant_id, user_id, "storefront", store_id, "storefront.update", serde_json::to_value(&storefront).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Storefront saved.", storefront)));
}

/// `GET /api/v1/storefronts/:store_id/shipping-zones`.
pub async fn list_zones(Path(store_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<ShippingZone>>>, ApiError> {
    assert_manage(&mut tx, store_id).await?;
    let zones = sqlx::query_as::<_, ShippingZone>(&format!("SELECT {ZONE_COLUMNS} FROM atlas.shipping_zones WHERE store_id = $1 ORDER BY name"))
        .bind(store_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shipping zones fetched.", zones)));
}

/// `POST /api/v1/storefronts/:store_id/shipping-zones`.
pub async fn create_zone(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ShippingZoneRequest>,
) -> Result<Json<ApiEnvelope<ShippingZone>>, ApiError> {
    assert_manage(&mut tx, store_id).await?;
    let name = required_text("name", &body.name, 100)?;
    if body.base_fee_minor < 0 || body.per_kg_fee_minor < 0 {
        return Err(ApiError::bad_request_with_code("fees must not be negative", "VALIDATION_FAILED"));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let regions: Vec<String> = body.regions.iter().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect();
    let zone = sqlx::query_as::<_, ShippingZone>(&format!(
        "INSERT INTO atlas.shipping_zones (id, tenant_id, store_id, name, regions, base_fee_minor, per_kg_fee_minor, allows_pay_on_delivery)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {ZONE_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(store_id)
    .bind(&name)
    .bind(&regions)
    .bind(body.base_fee_minor)
    .bind(body.per_kg_fee_minor)
    .bind(body.allows_pay_on_delivery)
    .fetch_one(&mut *tx)
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "shipping_zone", zone.id, "shipping_zone.create", serde_json::to_value(&zone).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shipping zone added.", zone)));
}

/// `PATCH /api/v1/storefronts/:store_id/shipping-zones/:zone_id`.
pub async fn update_zone(
    Path((store_id, zone_id)): Path<(Uuid, Uuid)>,
    mut tx: TenantTx,
    Json(body): Json<UpdateShippingZoneRequest>,
) -> Result<Json<ApiEnvelope<ShippingZone>>, ApiError> {
    assert_manage(&mut tx, store_id).await?;
    if body.base_fee_minor.is_some_and(|v| v < 0) || body.per_kg_fee_minor.is_some_and(|v| v < 0) {
        return Err(ApiError::bad_request_with_code("fees must not be negative", "VALIDATION_FAILED"));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let zone = sqlx::query_as::<_, ShippingZone>(&format!(
        "UPDATE atlas.shipping_zones SET
             name = COALESCE($3, name), regions = COALESCE($4, regions), base_fee_minor = COALESCE($5, base_fee_minor),
             per_kg_fee_minor = COALESCE($6, per_kg_fee_minor), allows_pay_on_delivery = COALESCE($7, allows_pay_on_delivery),
             is_active = COALESCE($8, is_active)
         WHERE id = $1 AND store_id = $2
         RETURNING {ZONE_COLUMNS}"
    ))
    .bind(zone_id)
    .bind(store_id)
    .bind(body.name.as_deref().map(str::trim))
    .bind(&body.regions)
    .bind(body.base_fee_minor)
    .bind(body.per_kg_fee_minor)
    .bind(body.allows_pay_on_delivery)
    .bind(body.is_active)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Shipping zone not found."))?;
    audit::record(&mut tx, tenant_id, user_id, "shipping_zone", zone.id, "shipping_zone.update", serde_json::to_value(&zone).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shipping zone updated.", zone)));
}
