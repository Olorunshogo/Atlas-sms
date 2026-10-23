use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::require_visible;
use crate::roles::service::StoreScope;

use super::models::{
    CreateCustomerRequest, Customer, HistoryQuery, ListCustomersQuery, Page, PurchaseHistoryEntry, UpdateCustomerRequest,
};
use super::service;

/**
 * Customers belong to the tenant, not a store, so every check runs against the caller's current store (`x-store-id`) - a store-scoped grant only matches when the request says which store the caller is working at.
 */
async fn assert_at_current_store(tx: &mut TenantTx, permission: &str) -> Result<(), ApiError> {
    let (user_id, tenant_id, store_id) = (tx.user_id, tx.tenant_id, tx.store_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, store_id, permission).await;
}

/**
 * A customer's group decides which price rules apply to them, so choosing one is a pricing decision, not part of the profile.
 * Needs the catalogue permission tenant-wide, and the group must belong to this tenant (foreign keys skip row level security).
 */
async fn assert_may_set_group(tx: &mut TenantTx, group_id: Option<Uuid>) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(tx, user_id, tenant_id, None, "product.manage").await?;
    if let Some(group_id) = group_id {
        require_visible(tx, "customer_groups", group_id, "Customer group").await?;
    }
    return Ok(());
}

/// Full contact details only for callers holding `customer.history`; everyone else gets the masked view.
async fn reveal_contact(tx: &mut TenantTx) -> Result<bool, ApiError> {
    let (user_id, tenant_id, store_id) = (tx.user_id, tx.tenant_id, tx.store_id);
    return roles::service::has_permission(tx, user_id, tenant_id, store_id, "customer.history").await;
}

fn present(customer: Customer, reveal: bool) -> Customer {
    return if reveal { customer } else { customer.masked() };
}

/**
 * `GET /api/v1/customers?q=&cursor=&limit=`.
 * Requires `customer.view`. Contact details are masked unless the caller also holds `customer.history`.
 */
pub async fn list_customers(
    mut tx: TenantTx,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<ApiEnvelope<Page<Customer>>>, ApiError> {
    assert_at_current_store(&mut tx, "customer.view").await?;
    let reveal = reveal_contact(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let customers = service::list_customers(&mut tx, tenant_id, query.q.as_deref(), query.cursor, limit).await?;
    tx.commit().await?;
    let next_cursor = if customers.len() as i64 == limit { customers.last().map(|c| c.id) } else { None };
    return Ok(Json(ApiEnvelope::ok(
        "Customers fetched.",
        Page {
            items: customers.into_iter().map(|c| present(c, reveal)).collect(),
            next_cursor,
        },
    )));
}

/// `GET /api/v1/customers/:id`. Same permission and masking as the list.
pub async fn get_customer(Path(customer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    assert_at_current_store(&mut tx, "customer.view").await?;
    let reveal = reveal_contact(&mut tx).await?;
    let customer = service::get_customer(&mut tx, customer_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer fetched.", present(customer, reveal))));
}

/**
 * `POST /api/v1/customers`.
 * Requires `customer.manage` - cashiers hold it, since registering a customer is routinely a checkout action. A phone number already on file is a 409 (`CUSTOMER_PHONE_EXISTS`) so the till looks the existing customer up instead of creating a duplicate.
 */
pub async fn create_customer(
    mut tx: TenantTx,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    assert_at_current_store(&mut tx, "customer.manage").await?;
    if body.customer_group_id.is_some() {
        assert_may_set_group(&mut tx, body.customer_group_id).await?;
    }
    let (tenant_id, user_id, store_id) = (tx.tenant_id, tx.user_id, tx.store_id);
    let customer = service::create_customer(&mut tx, tenant_id, user_id, store_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "customer", customer.id, "customer.create", serde_json::to_value(&customer).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer registered.", customer)));
}

/// `PATCH /api/v1/customers/:id`. Requires `customer.manage`; audited with the before and after state.
pub async fn update_customer(
    Path(customer_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiEnvelope<Customer>>, ApiError> {
    assert_at_current_store(&mut tx, "customer.manage").await?;
    if let Some(group_id) = body.customer_group_id {
        assert_may_set_group(&mut tx, group_id).await?;
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let before = service::get_customer(&mut tx, customer_id).await?;
    let customer = service::update_customer(&mut tx, customer_id, &body).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "customer",
            entity_id: customer.id,
            action: "customer.update",
            before: serde_json::to_value(&before).ok(),
            after: serde_json::to_value(&customer).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer updated.", customer)));
}

/**
 * `DELETE /api/v1/customers/:id`.
 * Requires `customer.manage`. Erases the person's details but keeps the row, so the sales that reference it still add up - see `service::erase_customer`.
 */
pub async fn delete_customer(Path(customer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    assert_at_current_store(&mut tx, "customer.manage").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    service::erase_customer(&mut tx, customer_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "customer", customer_id, "customer.erase", None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Customer deleted.", ())));
}

/**
 * `GET /api/v1/customers/:id/sales?cursor=&limit=`.
 * Requires `customer.history`, and only returns sales from stores where the caller holds it - the "eligible" purchase history.
 */
pub async fn purchase_history(
    Path(customer_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ApiEnvelope<Page<PurchaseHistoryEntry>>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let scope = roles::service::stores_with_permission(&mut tx, user_id, tenant_id, "customer.history").await?;
    if scope == StoreScope::Stores(vec![]) {
        return Err(ApiError::forbidden("Missing permission: customer.history"));
    }
    service::get_customer(&mut tx, customer_id).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let entries = service::purchase_history(&mut tx, customer_id, scope.as_filter(), query.cursor, limit).await?;
    tx.commit().await?;
    let next_cursor = if entries.len() as i64 == limit { entries.last().map(|e| e.sale_id) } else { None };
    return Ok(Json(ApiEnvelope::ok("Purchase history fetched.", Page { items: entries, next_cursor })));
}
