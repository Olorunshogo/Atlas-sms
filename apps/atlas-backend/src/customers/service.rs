use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{Customer, UpdateCustomerRequest};

const CUSTOMER_COLUMNS: &str =
    "id, full_name, phone, email, customer_group_id, store_credit_amount_minor";

/**
 * `query` is matched with a bare `ILIKE '%...%'` across name/phone/email, not a real search index (`pg_trgm`, full-text).
 * Fine at the row counts a single tenant's customer list will actually reach; revisit if that stops being true.
 */
pub async fn list_customers(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    query: Option<&str>,
) -> Result<Vec<Customer>, ApiError> {
    let pattern = query.map(|q| format!("%{}%", q));
    return sqlx::query_as::<_, Customer>(&format!(
        "SELECT {} FROM atlas.customers
         WHERE tenant_id = $1 AND deleted_at IS NULL
           AND ($2::text IS NULL OR full_name ILIKE $2 OR phone ILIKE $2 OR email ILIKE $2)
         ORDER BY full_name",
        CUSTOMER_COLUMNS
    ))
    .bind(tenant_id)
    .bind(pattern)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_customer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    full_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    customer_group_id: Option<Uuid>,
) -> Result<Customer, ApiError> {
    return sqlx::query_as::<_, Customer>(&format!(
        "INSERT INTO atlas.customers (id, tenant_id, full_name, phone, email, customer_group_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {}",
        CUSTOMER_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(full_name)
    .bind(phone)
    .bind(email)
    .bind(customer_group_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn update_customer(
    conn: &mut PgConnection,
    customer_id: Uuid,
    payload: &UpdateCustomerRequest,
) -> Result<Customer, ApiError> {
    return sqlx::query_as::<_, Customer>(&format!(
        "UPDATE atlas.customers
         SET full_name = COALESCE($2, full_name),
             phone = COALESCE($3, phone),
             email = COALESCE($4, email),
             customer_group_id = COALESCE($5, customer_group_id)
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {}",
        CUSTOMER_COLUMNS
    ))
    .bind(customer_id)
    .bind(&payload.full_name)
    .bind(&payload.phone)
    .bind(&payload.email)
    .bind(payload.customer_group_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Customer not found.".to_string()));
}
