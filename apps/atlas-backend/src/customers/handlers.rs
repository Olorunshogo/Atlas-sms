use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{Customer, CreateCustomerRequest, ListCustomersQuery, UpdateCustomerRequest};
use super::service;

/**
 * `GET /api/v1/customers?query=`.
 * Tenant-wide visibility, same reasoning as `stores`/`products` - no permission check beyond being a member of the tenant.
 */
pub async fn list_customers(
    mut tx: TenantTx,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<ApiEnvelope<Vec<Customer>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let customers = service::list_customers(&mut tx, tenant_id, query.query.as_deref()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customers fetched.", customers)));
}

/**
 * `POST /api/v1/customers`.
 * Gated on `sale.create`, not a dedicated customer permission (none exists in the permission enum today) - registering a customer is routinely a cashier action at checkout, not a management one, so it rides on the same permission that lets them ring up the sale.
 */
pub async fn create_customer(
    mut tx: TenantTx,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "sale.create").await?;
    let customer = service::create_customer(
        &mut tx,
        tenant_id,
        &body.full_name,
        body.phone.as_deref(),
        body.email.as_deref(),
        body.customer_group_id,
    )
    .await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "customer",
        customer.id,
        "customer.create",
        serde_json::to_value(&customer).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer registered.", customer)));
}

/**
 * `PATCH /api/v1/customers/:id`.
 * Same `sale.create` reasoning as create.
 */
pub async fn update_customer(
    Path(customer_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "sale.create").await?;
    let customer = service::update_customer(&mut tx, customer_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "customer",
        customer.id,
        "customer.update",
        serde_json::to_value(&customer).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer updated.", customer)));
}
