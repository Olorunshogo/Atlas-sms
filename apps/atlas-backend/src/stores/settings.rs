/*!
 * The per-store sales configuration: tax treatment, rounding, discount,
 * stock, pay-later and returns policy, and what the receipt prints about
 * the branch. Read by anyone selling at the store, changed only with
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
     cash_rounding_increment_minor, max_cashier_discount_basis_points, allow_negative_stock, allow_pay_later,
     address, phone, receipt_prefix, receipt_header, receipt_footer, return_window_days,
     refund_approval_threshold_minor, restockable_conditions::text[] AS restockable_conditions,
     allow_expired_sale, expiry_warning_days";

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
    /// Lets a sale with a named customer complete before it's fully paid.
    pub allow_pay_later: bool,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub receipt_prefix: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
    /// Whole local days after the sale in which a return needs no approval.
    pub return_window_days: i32,
    /// A refund at or above this needs approval; `None` means no amount limit.
    pub refund_approval_threshold_minor: Option<i64>,
    /// Item conditions that go back into saleable stock on return.
    pub restockable_conditions: Vec<String>,
    /// Lets an expired batch be sold; off, first-expiry-first-out skips it and naming it is refused (FR-14).
    pub allow_expired_sale: bool,
    /// How many days ahead of expiry a batch shows as near expiry.
    pub expiry_warning_days: i32,
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
    pub allow_pay_later: Option<bool>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub receipt_prefix: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
    pub return_window_days: Option<i32>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub refund_approval_threshold_minor: Option<Option<i64>>,
    pub restockable_conditions: Option<Vec<String>>,
    pub allow_expired_sale: Option<bool>,
    pub expiry_warning_days: Option<i32>,
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
    if body.expiry_warning_days.is_some_and(|v| v < 0) {
        return Err(ApiError::bad_request("expiry_warning_days must not be negative"));
    }
    if body.return_window_days.is_some_and(|v| v < 0) {
        return Err(ApiError::bad_request("return_window_days must not be negative"));
    }
    if body.refund_approval_threshold_minor.flatten().is_some_and(|v| v <= 0) {
        return Err(ApiError::bad_request("refund_approval_threshold_minor must be positive"));
    }
    if let Some(conditions) = &body.restockable_conditions {
        if let Some(bad) = conditions.iter().find(|c| !crate::returns::models::CONDITIONS.contains(&c.as_str())) {
            return Err(ApiError::bad_request(format!("unknown item condition: {}", bad)));
        }
    }
    let before = fetch(&mut tx, store_id).await?;

    sqlx::query(
        "UPDATE atlas.stores
         SET prices_include_tax = COALESCE($2, prices_include_tax),
             default_tax_rate_id = CASE WHEN $3 THEN $4 ELSE default_tax_rate_id END,
             cash_rounding_increment_minor = COALESCE($5, cash_rounding_increment_minor),
             max_cashier_discount_basis_points = COALESCE($6, max_cashier_discount_basis_points),
             allow_negative_stock = COALESCE($7, allow_negative_stock),
             allow_pay_later = COALESCE($8, allow_pay_later),
             address = COALESCE($9, address),
             phone = COALESCE($10, phone),
             receipt_prefix = COALESCE($11, receipt_prefix),
             receipt_header = COALESCE($12, receipt_header),
             receipt_footer = COALESCE($13, receipt_footer),
             return_window_days = COALESCE($14, return_window_days),
             refund_approval_threshold_minor = CASE WHEN $15 THEN $16 ELSE refund_approval_threshold_minor END,
             restockable_conditions = COALESCE($17::atlas.item_condition[], restockable_conditions),
             allow_expired_sale = COALESCE($18, allow_expired_sale),
             expiry_warning_days = COALESCE($19, expiry_warning_days),
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
    .bind(body.allow_pay_later)
    .bind(&body.address)
    .bind(&body.phone)
    .bind(body.receipt_prefix.as_deref().map(|p| p.trim().to_uppercase()))
    .bind(&body.receipt_header)
    .bind(&body.receipt_footer)
    .bind(body.return_window_days)
    .bind(body.refund_approval_threshold_minor.is_some())
    .bind(body.refund_approval_threshold_minor.flatten())
    .bind(&body.restockable_conditions)
    .bind(body.allow_expired_sale)
    .bind(body.expiry_warning_days)
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
