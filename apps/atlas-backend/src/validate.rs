use serde::{Deserialize, Deserializer};

use crate::error::ApiError;

// === Patch bodies

/**
 * For PATCH fields where "absent" and "null" mean different things: absent
 * leaves the column alone (`None`), an explicit `null` clears it
 * (`Some(None)`), a value sets it (`Some(Some(v))`). Use with
 * `#[serde(default, deserialize_with = "double_option")]`.
 */
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

pub fn nothing_to_update() -> ApiError {
    ApiError::bad_request_with_code("no fields to update", "VALIDATION_FAILED")
}

// === Text

/**
 * Trims `value` and rejects it if nothing is left or it is longer than
 * `max` characters. Returns the trimmed text so callers store what they
 * validated.
 */
pub fn required_text(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request_with_code(
            format!("{field} is required"),
            "VALIDATION_FAILED",
        ));
    }
    if trimmed.chars().count() > max {
        return Err(ApiError::bad_request_with_code(
            format!("{field} must be at most {max} characters"),
            "VALIDATION_FAILED",
        ));
    }
    Ok(trimmed.to_string())
}

/// Lowercase letters, digits and single hyphens inside; 1 to 63 characters.
pub fn slug(field: &str, value: &str) -> Result<String, ApiError> {
    let slug = required_text(field, value, 63)?;
    let valid = !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(ApiError::bad_request_with_code(
            format!("{field} may only contain lowercase letters, digits and single hyphens"),
            "VALIDATION_FAILED",
        ));
    }
    Ok(slug)
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
    fn double_option_tells_absent_from_null() {
        let absent: Patch = serde_json::from_str("{}").unwrap();
        let cleared: Patch = serde_json::from_str(r#"{"sku":null}"#).unwrap();
        let set: Patch = serde_json::from_str(r#"{"sku":"A-1"}"#).unwrap();
        assert_eq!(absent.sku, None);
        assert_eq!(cleared.sku, Some(None));
        assert_eq!(set.sku, Some(Some("A-1".to_string())));
    }

    #[test]
    fn required_text_trims_and_bounds() {
        assert_eq!(required_text("name", "  Rice  ", 10).unwrap(), "Rice");
        assert!(required_text("name", "   ", 10).is_err());
        assert!(required_text("name", "", 10).is_err());
        assert!(required_text("name", "abcdef", 5).is_err());
        assert!(required_text("name", "abcde", 5).is_ok());
    }

    #[test]
    fn slug_accepts_only_clean_lowercase() {
        assert_eq!(slug("slug", "location-one").unwrap(), "location-one");
        assert_eq!(slug("slug", "store2").unwrap(), "store2");
        for bad in ["Location One", "-x", "x-", "a--b", "a_b", "ÅB", "", "  "] {
            assert!(slug("slug", bad).is_err(), "{bad:?} should be rejected");
        }
        assert!(slug("slug", &"a".repeat(64)).is_err());
    }
}
