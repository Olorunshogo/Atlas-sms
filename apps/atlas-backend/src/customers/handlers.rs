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
 * Requires `customer.view`, checked against the caller's current store (`x-store-id`) so a store-scoped cashier's grant can match.
 */
pub async fn list_customers(
    mut tx: TenantTx,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<ApiEnvelope<Vec<Customer>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = tx.store_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, store_id, "customer.view").await?;
    let customers = service::list_customers(&mut tx, tenant_id, query.query.as_deref()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customers fetched.", customers)));
}

/**
 * `POST /api/v1/customers`.
 * Requires `customer.manage` at the caller's current store - cashiers hold it, since registering a customer is routinely a checkout action.
 */
pub async fn create_customer(
    mut tx: TenantTx,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = tx.store_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, store_id, "customer.manage").await?;
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
 * Same `customer.manage` gate as create.
 */
pub async fn update_customer(
    Path(customer_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = tx.store_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, store_id, "customer.manage").await?;
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
