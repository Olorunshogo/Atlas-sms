use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path, Query};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{CreatePriceRuleRequest, PriceRule};
use super::service;

#[derive(Deserialize)]
pub struct ListQuery {
    pub product_id: Option<Uuid>,
}

/// `GET /api/v1/price-rules?product_id=`. Requires `product.manage`.
pub async fn list(
    mut tx: TenantTx,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiEnvelope<Vec<PriceRule>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let rules = service::list(&mut tx, tenant_id, query.product_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Price rules fetched.", rules)))
}

/// `POST /api/v1/price-rules`. Requires `product.manage`.
pub async fn create(
    mut tx: TenantTx,
    Json(body): Json<CreatePriceRuleRequest>,
) -> Result<Json<ApiEnvelope<PriceRule>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let rule = service::create(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Price rule created.", rule)))
}

/// `DELETE /api/v1/price-rules/{id}`. Requires `product.manage`.
pub async fn delete(
    mut tx: TenantTx,
    Path(rule_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    service::delete(&mut tx, rule_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Price rule deleted.", ())))
}
