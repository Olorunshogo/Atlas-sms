use serde::{Deserialize, Deserializer};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;

// === Text

/**
 * Trims `value` and rejects it if nothing is left or it is longer than `max` characters.
 * Returns the trimmed text so callers store what they validated.
 */
pub fn required_text(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request_with_code(format!("{} is required", field), "VALIDATION_FAILED"));
    }
    if trimmed.chars().count() > max {
        return Err(ApiError::bad_request_with_code(
            format!("{} must be at most {} characters", field, max),
            "VALIDATION_FAILED",
        ));
    }
    return Ok(trimmed.to_string());
}

/// Blank becomes `None`; otherwise trimmed and bounded.
pub fn optional_text(field: &str, value: &str, max: usize) -> Result<Option<String>, ApiError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    return required_text(field, value, max).map(Some);
}

/// Lowercase letters, digits and single hyphens inside; 1 to 63 characters.
pub fn slug(field: &str, value: &str) -> Result<String, ApiError> {
    let slug = required_text(field, value, 63)?;
    let valid = !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(ApiError::bad_request_with_code(
            format!("{} may only contain lowercase letters, digits and single hyphens", field),
            "VALIDATION_FAILED",
        ));
    }
    return Ok(slug);
}

// === Tenant ownership

/**
 * Foreign keys do not go through row level security, so a tenant could otherwise point a new record at another tenant's row.
 * The check carries an explicit `tenant_id` predicate rather than leaning on row level security alone: RLS is the seatbelt, and a connection that bypasses it (a superuser, as the integration tests use) must not turn a foreign id into a valid one.
 * `table` is always a literal from the caller, never request input.
 */
pub async fn require_visible_for(conn: &mut PgConnection, tenant_id: Uuid, table: &str, id: Uuid, label: &str) -> Result<(), ApiError> {
    let found: Option<i32> = sqlx::query_scalar(&format!("SELECT 1 FROM atlas.{} WHERE id = $1 AND tenant_id = $2", table))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(conn)
        .await?;
    return found
        .map(|_| ())
        .ok_or_else(|| ApiError::not_found(format!("{} not found.", label)));
}

/// [`require_visible_for`] for a handler that already holds a `TenantTx` and so knows its tenant.
pub async fn require_visible(tx: &mut TenantTx, table: &str, id: Uuid, label: &str) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id;
    return require_visible_for(tx, tenant_id, table, id, label).await;
}

// === Patch bodies

/**
 * For PATCH fields where "absent" and "null" mean different things: absent leaves the column alone (`None`), an explicit `null` clears it (`Some(None)`), a value sets it (`Some(Some(v))`).
 * Use with `#[serde(default, deserialize_with = "double_option")]`.
 */
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    return Option::<T>::deserialize(deserializer).map(Some);
}

pub fn nothing_to_update() -> ApiError {
    return ApiError::bad_request_with_code("no fields to update", "VALIDATION_FAILED");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Patch {
        #[serde(default, deserialize_with = "double_option")]
        sku: Option<Option<String>>,
    }

    #[test]
    fn required_text_trims_and_bounds() {
        assert_eq!(required_text("name", "  Rice  ", 10).unwrap(), "Rice");
        assert!(required_text("name", "   ", 10).is_err());
        assert!(required_text("name", "abcdef", 5).is_err());
        assert!(required_text("name", "abcde", 5).is_ok());
    }

    #[test]
    fn optional_text_treats_blank_as_absent() {
        assert_eq!(optional_text("address", "   ", 10).unwrap(), None);
        assert_eq!(optional_text("address", " 1 High St ", 20).unwrap(), Some("1 High St".to_string()));
        assert!(optional_text("address", &"x".repeat(21), 20).is_err());
    }

    #[test]
    fn slug_accepts_only_clean_lowercase() {
        assert_eq!(slug("slug", "location-one").unwrap(), "location-one");
        for bad in ["Location One", "-x", "x-", "a--b", "a_b", "ÅB", "", "  "] {
            assert!(slug("slug", bad).is_err(), "{:?} should be rejected", bad);
        }
        assert!(slug("slug", &"a".repeat(64)).is_err());
    }

    #[test]
    fn double_option_tells_absent_from_null() {
        let absent: Patch = serde_json::from_str("{}").unwrap();
        let cleared: Patch = serde_json::from_str(r#"{"sku":null}"#).unwrap();
        let set: Patch = serde_json::from_str(r#"{"sku":"A-1"}"#).unwrap();
        assert_eq!(absent.sku, None);
        assert_eq!(cleared.sku, Some(None));
        assert_eq!(set.sku, Some(Some("A-1".to_string())));
    }
}
