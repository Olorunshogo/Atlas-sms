/*!
 * Customer groups are pricing tiers: a price rule can target a group, and every customer in it is charged that price.
 * Reading groups is open to any member of the tenant; creating, renaming and deleting them is a catalogue decision (`product.manage`).
 */
use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{require_visible, required_text};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CustomerGroup {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomerGroupRequest {
    pub name: String,
}

async fn assert_manage(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await;
}

/// `GET /api/v1/customer-groups`.
pub async fn list_groups(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<CustomerGroup>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let groups = sqlx::query_as::<_, CustomerGroup>("SELECT id, name FROM atlas.customer_groups WHERE tenant_id = $1 ORDER BY name")
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer groups fetched.", groups)));
}

/// `POST /api/v1/customer-groups`. Requires `product.manage`. A duplicate name in the tenant is a 409.
pub async fn create_group(mut tx: TenantTx, Json(body): Json<CustomerGroupRequest>) -> Result<Json<ApiEnvelope<CustomerGroup>>, ApiError> {
    assert_manage(&mut tx).await?;
    let name = required_text("name", &body.name, 100)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let group = sqlx::query_as::<_, CustomerGroup>("INSERT INTO atlas.customer_groups (id, tenant_id, name) VALUES ($1, $2, $3) RETURNING id, name")
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
    audit::record(&mut tx, tenant_id, user_id, "customer_group", group.id, "customer_group.create", serde_json::to_value(&group).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer group created.", group)));
}

/// `PATCH /api/v1/customer-groups/:id`. Renames. Requires `product.manage`.
pub async fn rename_group(
    Path(group_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CustomerGroupRequest>,
) -> Result<Json<ApiEnvelope<CustomerGroup>>, ApiError> {
    assert_manage(&mut tx).await?;
    let name = required_text("name", &body.name, 100)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let before = audit::snapshot(&mut tx, "customer_groups", "id", group_id).await?;
    let group = sqlx::query_as::<_, CustomerGroup>("UPDATE atlas.customer_groups SET name = $2 WHERE id = $1 AND tenant_id = $3 RETURNING id, name")
        .bind(group_id)
        .bind(name)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("Customer group not found."))?;
    audit::record_edit(
        &mut tx,
        audit::Edit { tenant_id, actor_user_id: user_id, entity_type: "customer_group", entity_id: group.id, action: "customer_group.rename", table: "customer_groups", key: "id", before },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer group renamed.", group)));
}

/**
 * `DELETE /api/v1/customer-groups/:id`.
 * Requires `product.manage`; refused while any customer or price rule still points at the group.
 * The price rules would otherwise be removed silently by the foreign key's cascade, quietly changing what those customers are charged.
 */
pub async fn delete_group(Path(group_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    assert_manage(&mut tx).await?;
    require_visible(&mut tx, "customer_groups", group_id, "Customer group").await?;
    let in_use: Option<i32> = sqlx::query_scalar(
        "SELECT 1 WHERE EXISTS (SELECT 1 FROM atlas.customers WHERE customer_group_id = $1)
                     OR EXISTS (SELECT 1 FROM atlas.price_rules WHERE customer_group_id = $1)",
    )
    .bind(group_id)
    .fetch_optional(&mut *tx)
    .await?;
    if in_use.is_some() {
        return Err(ApiError::conflict_with_code(
            "This group still has customers or price rules; move or remove them first.",
            "GROUP_IN_USE",
        ));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    sqlx::query("DELETE FROM atlas.customer_groups WHERE id = $1")
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    audit::record(&mut tx, tenant_id, user_id, "customer_group", group_id, "customer_group.delete", None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer group deleted.", ())));
}
