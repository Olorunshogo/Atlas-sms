use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};

use super::models::{CreateCustomerRequest, Customer, PurchaseHistoryEntry, UpdateCustomerRequest};

const CUSTOMER_COLUMNS: &str = "id, full_name, phone, email, address, notes, tax_identification_number,
     customer_group_id, store_credit_amount_minor, marketing_consent, home_store_id, created_at, updated_at";

/**
 * One canonical form per Nigerian number so "0803 123 4567", "+234 803 123 4567" and "2348031234567" are the same customer.
 * Anything that isn't recognisably Nigerian keeps its digits (and a leading `+` if it had one) rather than being guessed at.
 */
pub fn normalize_phone(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    if digits.len() == 11 && digits.starts_with('0') {
        return Some(format!("+234{}", &digits[1..]));
    }
    if digits.starts_with("234") || trimmed.starts_with('+') {
        return Some(format!("+{}", digits));
    }
    return Some(digits);
}

fn blank_to_none(value: Option<&str>) -> Option<String> {
    return value.map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
}

fn map_phone_conflict(err: sqlx::Error) -> ApiError {
    return match err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("A customer with this phone number already exists.", codes::CUSTOMER_PHONE_EXISTS)
        }
        other => ApiError::from(other),
    };
}

/**
 * `q` is a bare `ILIKE '%...%'` across name, phone and email plus an exact match on the normalised phone, not a real search index - fine at one tenant's customer-list size.
 * Keyset-paginated on (full_name, id): `cursor` is the last id of the previous page.
 */
pub async fn list_customers(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    q: Option<&str>,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Customer>, ApiError> {
    let term = q.map(str::trim).filter(|t| !t.is_empty());
    let pattern = term.map(|t| format!("%{}%", t));
    let phone = term.and_then(normalize_phone);
    return sqlx::query_as::<_, Customer>(&format!(
        "SELECT {} FROM atlas.customers
         WHERE tenant_id = $1 AND deleted_at IS NULL
           AND ($2::text IS NULL OR full_name ILIKE $2 OR phone ILIKE $2 OR email ILIKE $2 OR phone_normalized = $3)
           AND ($4::uuid IS NULL OR (full_name, id) > (SELECT full_name, id FROM atlas.customers WHERE id = $4))
         ORDER BY full_name, id
         LIMIT $5",
        CUSTOMER_COLUMNS
    ))
    .bind(tenant_id)
    .bind(pattern)
    .bind(phone)
    .bind(cursor)
    .bind(limit)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn get_customer(conn: &mut PgConnection, customer_id: Uuid) -> Result<Customer, ApiError> {
    return sqlx::query_as::<_, Customer>(&format!(
        "SELECT {} FROM atlas.customers WHERE id = $1 AND deleted_at IS NULL",
        CUSTOMER_COLUMNS
    ))
    .bind(customer_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Customer not found."));
}

pub async fn create_customer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    created_by_user_id: Uuid,
    home_store_id: Option<Uuid>,
    payload: &CreateCustomerRequest,
) -> Result<Customer, ApiError> {
    let full_name = payload.full_name.trim();
    if full_name.is_empty() {
        return Err(ApiError::bad_request("full_name is required"));
    }
    let phone = blank_to_none(payload.phone.as_deref());
    return sqlx::query_as::<_, Customer>(&format!(
        "INSERT INTO atlas.customers
            (id, tenant_id, full_name, phone, phone_normalized, email, address, notes, tax_identification_number,
             customer_group_id, marketing_consent, consent_recorded_at, home_store_id, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CASE WHEN $11 THEN now() END, $12, $13)
         RETURNING {}",
        CUSTOMER_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(full_name)
    .bind(&phone)
    .bind(phone.as_deref().and_then(normalize_phone))
    .bind(blank_to_none(payload.email.as_deref()))
    .bind(blank_to_none(payload.address.as_deref()))
    .bind(blank_to_none(payload.notes.as_deref()))
    .bind(blank_to_none(payload.tax_identification_number.as_deref()))
    .bind(payload.customer_group_id)
    .bind(payload.marketing_consent)
    .bind(home_store_id)
    .bind(created_by_user_id)
    .fetch_one(conn)
    .await
    .map_err(map_phone_conflict);
}

pub async fn update_customer(
    conn: &mut PgConnection,
    customer_id: Uuid,
    payload: &UpdateCustomerRequest,
) -> Result<Customer, ApiError> {
    if payload.full_name.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err(ApiError::bad_request("full_name must not be empty"));
    }
    let phone = payload.phone.as_ref().map(|p| blank_to_none(p.as_deref()));
    let text = |field: &Option<Option<String>>| field.as_ref().map(|v| blank_to_none(v.as_deref()));
    let (email, address, notes, tin) = (text(&payload.email), text(&payload.address), text(&payload.notes), text(&payload.tax_identification_number));

    return sqlx::query_as::<_, Customer>(&format!(
        "UPDATE atlas.customers
         SET full_name = COALESCE($2, full_name),
             phone = CASE WHEN $3 THEN $4 ELSE phone END,
             phone_normalized = CASE WHEN $3 THEN $5 ELSE phone_normalized END,
             email = CASE WHEN $6 THEN $7 ELSE email END,
             address = CASE WHEN $8 THEN $9 ELSE address END,
             notes = CASE WHEN $10 THEN $11 ELSE notes END,
             tax_identification_number = CASE WHEN $12 THEN $13 ELSE tax_identification_number END,
             customer_group_id = CASE WHEN $14 THEN $15 ELSE customer_group_id END,
             consent_recorded_at = CASE WHEN $16 IS NOT NULL AND $16 IS DISTINCT FROM marketing_consent THEN now() ELSE consent_recorded_at END,
             marketing_consent = COALESCE($16, marketing_consent),
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {}",
        CUSTOMER_COLUMNS
    ))
    .bind(customer_id)
    .bind(payload.full_name.as_deref().map(str::trim))
    .bind(phone.is_some())
    .bind(phone.clone().flatten())
    .bind(phone.flatten().as_deref().and_then(normalize_phone))
    .bind(email.is_some())
    .bind(email.flatten())
    .bind(address.is_some())
    .bind(address.flatten())
    .bind(notes.is_some())
    .bind(notes.flatten())
    .bind(tin.is_some())
    .bind(tin.flatten())
    .bind(payload.customer_group_id.is_some())
    .bind(payload.customer_group_id.flatten())
    .bind(payload.marketing_consent)
    .fetch_optional(conn)
    .await
    .map_err(map_phone_conflict)?
    .ok_or_else(|| ApiError::not_found("Customer not found."));
}

/**
 * Soft delete that also erases personal data: the row stays (past sales still point at it, so revenue history keeps adding up) but nothing on it identifies a person any more.
 */
pub async fn erase_customer(conn: &mut PgConnection, customer_id: Uuid) -> Result<(), ApiError> {
    let erased = sqlx::query(
        "UPDATE atlas.customers
         SET full_name = 'Deleted customer', phone = NULL, phone_normalized = NULL, email = NULL,
             address = NULL, notes = NULL, tax_identification_number = NULL, marketing_consent = false,
             consent_recorded_at = NULL, deleted_at = now(), updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(customer_id)
    .execute(conn)
    .await?;
    if erased.rows_affected() == 0 {
        return Err(ApiError::not_found("Customer not found."));
    }
    return Ok(());
}

/// A customer's sales, newest first, restricted to `stores` (`None` = every store). Keyset on (created_at, id) via `cursor`.
pub async fn purchase_history(
    conn: &mut PgConnection,
    customer_id: Uuid,
    stores: Option<Vec<Uuid>>,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<PurchaseHistoryEntry>, ApiError> {
    return sqlx::query_as::<_, PurchaseHistoryEntry>(
        "SELECT s.id AS sale_id, s.receipt_number, s.store_id, st.name AS store_name, s.status::text AS status,
                s.payment_status::text AS payment_status, s.total_amount_minor, s.amount_refunded_minor, s.created_at
         FROM atlas.sales s
         JOIN atlas.stores st ON st.id = s.store_id
         WHERE s.customer_id = $1
           AND ($2::uuid[] IS NULL OR s.store_id = ANY($2))
           AND ($3::uuid IS NULL OR (s.created_at, s.id) < (SELECT created_at, id FROM atlas.sales WHERE id = $3))
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT $4",
    )
    .bind(customer_id)
    .bind(stores)
    .bind(cursor)
    .bind(limit)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

#[cfg(test)]
mod tests {
    use super::normalize_phone;

    #[test]
    fn nigerian_numbers_share_one_normal_form() {
        let expected = Some("+2348031234567".to_string());
        assert_eq!(normalize_phone("0803 123 4567"), expected);
        assert_eq!(normalize_phone("+234 803-123-4567"), expected);
        assert_eq!(normalize_phone("2348031234567"), expected);
        assert_eq!(normalize_phone("  "), None);
        assert_eq!(normalize_phone("+44 20 7946 0958"), Some("+442079460958".to_string()));
    }
}
