use axum::extract::Path;
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{CreateTaxRateRequest, TaxRate, UpdateTaxRateRequest};

const TAX_RATE_COLUMNS: &str = "id, name, rate_basis_points, is_active";

/**
 * `GET /api/v1/tax-rates`.
 * Tenant-wide visibility, same as products - every till needs to show the rate it charged.
 */
pub async fn list_tax_rates(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<TaxRate>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let rates = sqlx::query_as::<_, TaxRate>(&format!(
        "SELECT {} FROM atlas.tax_rates WHERE tenant_id = $1 ORDER BY name",
        TAX_RATE_COLUMNS
    ))
    .bind(tenant_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Tax rates fetched.", rates)));
}

/**
 * `POST /api/v1/tax-rates`.
 * Requires `product.manage`. A rate's percentage is never edited after creation - a rate change is a new row, so historical sales keep pointing at the rate they were charged.
 */
pub async fn create_tax_rate(
    mut tx: TenantTx,
    Json(body): Json<CreateTaxRateRequest>,
) -> Result<Json<ApiEnvelope<TaxRate>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    if !(0..=10_000).contains(&body.rate_basis_points) {
        return Err(ApiError::bad_request("rate_basis_points must be between 0 and 10000"));
    }
    let rate = sqlx::query_as::<_, TaxRate>(&format!(
        "INSERT INTO atlas.tax_rates (id, tenant_id, name, rate_basis_points)
         VALUES ($1, $2, $3, $4)
         RETURNING {}",
        TAX_RATE_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&body.name)
    .bind(body.rate_basis_points)
    .fetch_one(&mut *tx)
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "tax_rate", rate.id, "tax_rate.create", serde_json::to_value(&rate).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Tax rate created.", rate)));
}

/// `PATCH /api/v1/tax-rates/:id` - rename or retire a rate. Requires `product.manage`.
pub async fn update_tax_rate(
    Path(rate_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateTaxRateRequest>,
) -> Result<Json<ApiEnvelope<TaxRate>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let before = audit::snapshot(&mut tx, "tax_rates", "id", rate_id).await?;
    let rate = sqlx::query_as::<_, TaxRate>(&format!(
        "UPDATE atlas.tax_rates
         SET name = COALESCE($2, name), is_active = COALESCE($3, is_active), updated_at = now()
         WHERE id = $1
         RETURNING {}",
        TAX_RATE_COLUMNS
    ))
    .bind(rate_id)
    .bind(&body.name)
    .bind(body.is_active)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Tax rate not found."))?;
    audit::record_edit(
        &mut tx,
        audit::Edit { tenant_id, actor_user_id: user_id, entity_type: "tax_rate", entity_id: rate.id, action: "tax_rate.update", table: "tax_rates", key: "id", before },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Tax rate updated.", rate)));
}
