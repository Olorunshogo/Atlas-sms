use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::validate::{nothing_to_update, required_text};

use super::models::{
    CreateCustomerRequest, Customer, CustomerGroup, CustomerGroupRequest, UpdateCustomerRequest,
};

const CUSTOMER_SELECT: &str = "SELECT c.id, c.full_name, c.phone, c.email, c.customer_group_id,
        g.name AS customer_group_name, c.store_credit_amount_minor, c.created_at
    FROM atlas.customers c
    LEFT JOIN atlas.customer_groups g ON g.id = c.customer_group_id";

// === Permission

/**
 * True if the user holds `permission` on at least one grant in this tenant,
 * at any scope. Customers belong to the tenant, not a store, but cashiers
 * only ever hold store-scoped grants, so "tenant-wide permission" would
 * lock out the very staff who serve customers.
 */
pub async fn holds_permission_anywhere(
    conn: &mut PgConnection,
    user_id: Uuid,
    tenant_id: Uuid,
    permission: &str,
) -> Result<bool, ApiError> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1
         FROM atlas.role_grants rg
         JOIN atlas.roles r ON r.id = rg.role_id
         WHERE rg.user_id = $1
           AND rg.status = 'active'
           AND $3::atlas.permission = ANY(r.permissions)
           AND (rg.scope = 'platform' OR rg.tenant_id = $2)
         LIMIT 1",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(permission)
    .fetch_optional(conn)
    .await?;
    Ok(found.is_some())
}

// === Validation

/** Blank becomes `None`; otherwise trimmed and bounded. */
fn optional_text(field: &str, value: &str, max: usize) -> Result<Option<String>, ApiError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    required_text(field, value, max).map(Some)
}

fn clean_phone(value: &str) -> Result<Option<String>, ApiError> {
    let Some(phone) = optional_text("phone", value, 20)? else {
        return Ok(None);
    };
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    let allowed = phone
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
    if !allowed || !(7..=15).contains(&digits) {
        return Err(ApiError::bad_request_with_code(
            "phone must have 7 to 15 digits and may only use digits, spaces, + - ( )",
            "VALIDATION_FAILED",
        ));
    }
    Ok(Some(phone))
}

fn clean_email(value: &str) -> Result<Option<String>, ApiError> {
    let Some(email) = optional_text("email", value, 254)? else {
        return Ok(None);
    };
    let email = email.to_ascii_lowercase();
    let valid = match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !email.chars().any(char::is_whitespace)
        }
        None => false,
    };
    if !valid {
        return Err(ApiError::bad_request_with_code(
            "email is not a valid address",
            "VALIDATION_FAILED",
        ));
    }
    Ok(Some(email))
}

/** Group ids bypass RLS in foreign keys, so read one back through the scoped transaction. */
async fn require_group(conn: &mut PgConnection, group_id: Uuid) -> Result<(), ApiError> {
    let found: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM atlas.customer_groups WHERE id = $1")
            .bind(group_id)
            .fetch_optional(conn)
            .await?;
    found
        .map(|_| ())
        .ok_or_else(|| ApiError::NotFound("Customer group not found.".to_string()))
}

// === Customer groups

pub async fn list_groups(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Vec<CustomerGroup>, ApiError> {
    sqlx::query_as::<_, CustomerGroup>(
        "SELECT id, name FROM atlas.customer_groups WHERE tenant_id = $1 ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_group(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CustomerGroupRequest,
) -> Result<CustomerGroup, ApiError> {
    let name = required_text("name", &payload.name, 100)?;
    sqlx::query_as::<_, CustomerGroup>(
        "INSERT INTO atlas.customer_groups (id, tenant_id, name) VALUES ($1, $2, $3)
         RETURNING id, name",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn rename_group(
    conn: &mut PgConnection,
    group_id: Uuid,
    payload: &CustomerGroupRequest,
) -> Result<CustomerGroup, ApiError> {
    let name = required_text("name", &payload.name, 100)?;
    sqlx::query_as::<_, CustomerGroup>(
        "UPDATE atlas.customer_groups SET name = $2 WHERE id = $1 RETURNING id, name",
    )
    .bind(group_id)
    .bind(name)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Customer group not found.".to_string()))
}

/**
 * Refused while anything points at the group. Price rules would otherwise
 * be deleted silently by the foreign key's cascade, quietly changing what
 * those customers are charged.
 */
pub async fn delete_group(conn: &mut PgConnection, group_id: Uuid) -> Result<(), ApiError> {
    require_group(conn, group_id).await?;
    let in_use: Option<i32> = sqlx::query_scalar(
        "SELECT 1 WHERE EXISTS (SELECT 1 FROM atlas.customers WHERE customer_group_id = $1)
                     OR EXISTS (SELECT 1 FROM atlas.price_rules WHERE customer_group_id = $1)",
    )
    .bind(group_id)
    .fetch_optional(&mut *conn)
    .await?;
    if in_use.is_some() {
        return Err(ApiError::conflict_with_code(
            "This group still has customers or price rules; move or remove them first.",
            "GROUP_IN_USE",
        ));
    }
    sqlx::query("DELETE FROM atlas.customer_groups WHERE id = $1")
        .bind(group_id)
        .execute(conn)
        .await?;
    Ok(())
}

// === Customers

pub async fn get_customer(
    conn: &mut PgConnection,
    customer_id: Uuid,
) -> Result<Customer, ApiError> {
    sqlx::query_as::<_, Customer>(&format!(
        "{CUSTOMER_SELECT} WHERE c.id = $1 AND c.deleted_at IS NULL"
    ))
    .bind(customer_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Customer not found.".to_string()))
}

/** `LIKE` treats `%`, `_` and `\` as wildcards; searching for them literally must still work. */
fn escape_like(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub async fn list_customers(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    search: Option<&str>,
    group_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Customer>, ApiError> {
    let pattern = search
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| format!("%{}%", escape_like(term)));
    sqlx::query_as::<_, Customer>(&format!(
        "{CUSTOMER_SELECT}
         WHERE c.tenant_id = $1 AND c.deleted_at IS NULL
           AND ($2::text IS NULL OR c.full_name ILIKE $2 OR c.phone ILIKE $2 OR c.email ILIKE $2)
           AND ($3::uuid IS NULL OR c.customer_group_id = $3)
         ORDER BY c.full_name, c.id
         LIMIT $4 OFFSET $5"
    ))
    .bind(tenant_id)
    .bind(pattern)
    .bind(group_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_customer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateCustomerRequest,
) -> Result<Customer, ApiError> {
    let full_name = required_text("full_name", &payload.full_name, 100)?;
    let phone = match payload.phone.as_deref() {
        Some(phone) => clean_phone(phone)?,
        None => None,
    };
    let email = match payload.email.as_deref() {
        Some(email) => clean_email(email)?,
        None => None,
    };
    if let Some(group_id) = payload.customer_group_id {
        require_group(conn, group_id).await?;
    }

    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.customers (id, tenant_id, full_name, phone, email, customer_group_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(full_name)
    .bind(phone)
    .bind(email)
    .bind(payload.customer_group_id)
    .execute(&mut *conn)
    .await?;
    get_customer(conn, id).await
}

pub async fn update_customer(
    conn: &mut PgConnection,
    customer_id: Uuid,
    payload: &UpdateCustomerRequest,
) -> Result<Customer, ApiError> {
    if payload.is_empty() {
        return Err(nothing_to_update());
    }
    let full_name = match payload.full_name.as_deref() {
        Some(name) => Some(required_text("full_name", name, 100)?),
        None => None,
    };
    let (set_phone, phone) = match &payload.phone {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(phone)) => (true, clean_phone(phone)?),
    };
    let (set_email, email) = match &payload.email {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(email)) => (true, clean_email(email)?),
    };
    let (set_group, group_id) = match payload.customer_group_id {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(group_id)) => {
            require_group(conn, group_id).await?;
            (true, Some(group_id))
        }
    };

    let result = sqlx::query(
        "UPDATE atlas.customers
         SET full_name = COALESCE($2, full_name),
             phone = CASE WHEN $3 THEN $4 ELSE phone END,
             email = CASE WHEN $5 THEN $6 ELSE email END,
             customer_group_id = CASE WHEN $7 THEN $8 ELSE customer_group_id END
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(customer_id)
    .bind(full_name)
    .bind(set_phone)
    .bind(phone)
    .bind(set_email)
    .bind(email)
    .bind(set_group)
    .bind(group_id)
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Customer not found.".to_string()));
    }
    get_customer(conn, customer_id).await
}

/** Soft delete: past sales keep pointing at the customer. */
pub async fn delete_customer(conn: &mut PgConnection, customer_id: Uuid) -> Result<(), ApiError> {
    let result = sqlx::query(
        "UPDATE atlas.customers SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(customer_id)
    .execute(conn)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Customer not found.".to_string()));
    }
    Ok(())
}
