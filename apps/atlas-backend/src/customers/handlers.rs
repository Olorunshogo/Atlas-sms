use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path, Query};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CreateCustomerRequest, Customer, CustomerGroup, CustomerGroupRequest, UpdateCustomerRequest,
};
use super::service;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

/// Staff who serve customers: anyone who can ring up a sale at some store.
async fn require_customer_staff(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if service::holds_permission_anywhere(tx, user_id, tenant_id, "sale.create").await? {
        Ok(())
    } else {
        Err(ApiError::forbidden("Missing permission: sale.create"))
    }
}

/// Catalogue-level changes (groups, and which group a customer is in).
async fn require_catalogue_manager(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await
}

// === Customer groups

/// `GET /api/v1/customer-groups`. Any member of the tenant.
pub async fn list_groups(
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<CustomerGroup>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let groups = service::list_groups(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer groups fetched.", groups)))
}

/// `POST /api/v1/customer-groups`. Requires `product.manage`: a group is a pricing tier.
pub async fn create_group(
    mut tx: TenantTx,
    Json(body): Json<CustomerGroupRequest>,
) -> Result<Json<ApiEnvelope<CustomerGroup>>, ApiError> {
    require_catalogue_manager(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let group = service::create_group(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer group created.", group)))
}

/// `PATCH /api/v1/customer-groups/{id}`. Renames. Requires `product.manage`.
pub async fn rename_group(
    mut tx: TenantTx,
    Path(group_id): Path<Uuid>,
    Json(body): Json<CustomerGroupRequest>,
) -> Result<Json<ApiEnvelope<CustomerGroup>>, ApiError> {
    require_catalogue_manager(&mut tx).await?;
    let group = service::rename_group(&mut tx, group_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer group renamed.", group)))
}

/// `DELETE /api/v1/customer-groups/{id}`. Requires `product.manage`; refused while in use.
pub async fn delete_group(
    mut tx: TenantTx,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    require_catalogue_manager(&mut tx).await?;
    service::delete_group(&mut tx, group_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer group deleted.", ())))
}

// === Customers

#[derive(Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub customer_group_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `GET /api/v1/customers?q=&customer_group_id=&limit=&offset=`. Requires `sale.create` somewhere.
pub async fn list_customers(
    mut tx: TenantTx,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiEnvelope<Vec<Customer>>>, ApiError> {
    require_customer_staff(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let customers = service::list_customers(
        &mut tx,
        tenant_id,
        query.q.as_deref(),
        query.customer_group_id,
        query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        query.offset.unwrap_or(0).max(0),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customers fetched.", customers)))
}

/// `GET /api/v1/customers/{id}`.
pub async fn get_customer(
    mut tx: TenantTx,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    require_customer_staff(&mut tx).await?;
    let customer = service::get_customer(&mut tx, customer_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer fetched.", customer)))
}

/// `POST /api/v1/customers`. Requires `sale.create` somewhere; choosing a group also needs `product.manage`.
pub async fn create_customer(
    mut tx: TenantTx,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    require_customer_staff(&mut tx).await?;
    if body.customer_group_id.is_some() {
        require_catalogue_manager(&mut tx).await?;
    }
    let tenant_id = tx.tenant_id;
    let customer = service::create_customer(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer created.", customer)))
}

/// `PATCH /api/v1/customers/{id}`. Same permissions as create.
pub async fn update_customer(
    mut tx: TenantTx,
    Path(customer_id): Path<Uuid>,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    require_customer_staff(&mut tx).await?;
    if body.customer_group_id.is_some() {
        require_catalogue_manager(&mut tx).await?;
    }
    let customer = service::update_customer(&mut tx, customer_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer updated.", customer)))
}

/// `DELETE /api/v1/customers/{id}`. Soft delete. Requires `sale.refund` somewhere (managers and up).
pub async fn delete_customer(
    mut tx: TenantTx,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if !service::holds_permission_anywhere(&mut tx, user_id, tenant_id, "sale.refund").await? {
        return Err(ApiError::forbidden("Missing permission: sale.refund"));
    }
    service::delete_customer(&mut tx, customer_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Customer deleted.", ())))
}
