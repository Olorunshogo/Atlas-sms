/*!
 * Product categories (FR-08): free-form merchandising groups, archived rather than deleted so products that used them stay readable.
 * Writes are gated on `product.manage`; reads are open to any member of the tenant.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{nothing_to_update, required_text};

const CATEGORY_COLUMNS: &str = "id, name, description, archived_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListCategoriesQuery {
    #[serde(default)]
    pub include_archived: bool,
}

async fn assert_manage(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await;
}

/// A product can only be put in a category that exists here and isn't archived.
pub async fn assert_assignable(conn: &mut PgConnection, tenant_id: Uuid, category_id: Uuid) -> Result<(), ApiError> {
    let archived: Option<bool> = sqlx::query_scalar("SELECT archived_at IS NOT NULL FROM atlas.product_categories WHERE id = $1 AND tenant_id = $2")
        .bind(category_id)
        .bind(tenant_id)
        .fetch_optional(conn)
        .await?;
    return match archived {
        None => Err(ApiError::not_found("Category not found.")),
        Some(true) => Err(ApiError::bad_request_with_code("Category is archived.", "VALIDATION_FAILED")),
        Some(false) => Ok(()),
    };
}

/// `GET /api/v1/product-categories?include_archived=`.
pub async fn list_categories(mut tx: TenantTx, Query(query): Query<ListCategoriesQuery>) -> Result<Json<ApiEnvelope<Vec<Category>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let categories = sqlx::query_as::<_, Category>(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM atlas.product_categories
         WHERE tenant_id = $1 AND ($2 OR archived_at IS NULL)
         ORDER BY name"
    ))
    .bind(tenant_id)
    .bind(query.include_archived)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Categories fetched.", categories)));
}

/// `POST /api/v1/product-categories`.
pub async fn create_category(mut tx: TenantTx, Json(body): Json<CreateCategoryRequest>) -> Result<Json<ApiEnvelope<Category>>, ApiError> {
    assert_manage(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let name = required_text("name", &body.name, 100)?;
    let category = sqlx::query_as::<_, Category>(&format!(
        "INSERT INTO atlas.product_categories (id, tenant_id, name, description) VALUES ($1, $2, $3, $4)
         RETURNING {CATEGORY_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&name)
    .bind(&body.description)
    .fetch_one(&mut *tx)
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "product_category", category.id, "product_category.create", serde_json::to_value(&category).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Category created.", category)));
}

/// `PATCH /api/v1/product-categories/:id`.
pub async fn update_category(
    Path(category_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateCategoryRequest>,
) -> Result<Json<ApiEnvelope<Category>>, ApiError> {
    assert_manage(&mut tx).await?;
    if body.name.is_none() && body.description.is_none() {
        return Err(nothing_to_update());
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let name = match body.name.as_deref() {
        Some(name) => Some(required_text("name", name, 100)?),
        None => None,
    };
    let before = audit::snapshot(&mut tx, "product_categories", "id", category_id).await?;
    let category = sqlx::query_as::<_, Category>(&format!(
        "UPDATE atlas.product_categories
         SET name = COALESCE($3, name), description = COALESCE($4, description), updated_at = now()
         WHERE id = $1 AND tenant_id = $2
         RETURNING {CATEGORY_COLUMNS}"
    ))
    .bind(category_id)
    .bind(tenant_id)
    .bind(&name)
    .bind(&body.description)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Category not found."))?;
    audit::record_edit(
        &mut tx,
        audit::Edit { tenant_id, actor_user_id: user_id, entity_type: "product_category", entity_id: category.id, action: "product_category.update", table: "product_categories", key: "id", before },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Category updated.", category)));
}

async fn set_archived(mut tx: TenantTx, category_id: Uuid, archived: bool) -> Result<Json<ApiEnvelope<Category>>, ApiError> {
    assert_manage(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let category = sqlx::query_as::<_, Category>(&format!(
        "UPDATE atlas.product_categories
         SET archived_at = CASE WHEN $3 THEN COALESCE(archived_at, now()) END, updated_at = now()
         WHERE id = $1 AND tenant_id = $2
         RETURNING {CATEGORY_COLUMNS}"
    ))
    .bind(category_id)
    .bind(tenant_id)
    .bind(archived)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Category not found."))?;
    let action = if archived { "product_category.archive" } else { "product_category.restore" };
    audit::record(&mut tx, tenant_id, user_id, "product_category", category.id, action, serde_json::to_value(&category).ok()).await?;
    tx.commit().await?;
    let message = if archived { "Category archived." } else { "Category restored." };
    return Ok(Json(ApiEnvelope::ok(message, category)));
}

/// `POST /api/v1/product-categories/:id/archive`. Products keep their `category_id`; it just can't be assigned to new ones.
pub async fn archive_category(Path(category_id): Path<Uuid>, tx: TenantTx) -> Result<Json<ApiEnvelope<Category>>, ApiError> {
    return set_archived(tx, category_id, true).await;
}

/// `POST /api/v1/product-categories/:id/restore`.
pub async fn restore_category(Path(category_id): Path<Uuid>, tx: TenantTx) -> Result<Json<ApiEnvelope<Category>>, ApiError> {
    return set_archived(tx, category_id, false).await;
}
