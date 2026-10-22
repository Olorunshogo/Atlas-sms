use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub tax_identification_number: Option<String>,
    pub customer_group_id: Option<Uuid>,
    pub store_credit_amount_minor: i64,
    pub marketing_consent: bool,
    pub home_store_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// True when contact details were masked because the caller lacks `customer.history`.
    #[sqlx(skip)]
    pub contact_masked: bool,
}

impl Customer {
    /**
     * Hides contact and free-text details from callers who may look a customer up (to attach them to a sale) but not see their profile.
     * The phone keeps its first four and last three digits so a cashier can still confirm "is this you?" at the till.
     */
    pub fn masked(mut self) -> Self {
        self.phone = self.phone.as_deref().map(mask_phone);
        self.email = self.email.as_deref().map(mask_email);
        self.address = self.address.map(|_| "***".to_string());
        self.notes = None;
        self.tax_identification_number = self.tax_identification_number.map(|_| "***".to_string());
        self.contact_masked = true;
        return self;
    }
}

fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() <= 7 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 3..].iter().collect();
    return format!("{}{}{}", head, "*".repeat(chars.len() - 7), tail);
}

fn mask_email(email: &str) -> String {
    return match email.split_once('@') {
        Some((local, domain)) => format!("{}***@{}", local.chars().next().unwrap_or('*'), domain),
        None => "***".to_string(),
    };
}

/// Distinguishes a field sent as `null` (clear it) from one left out (keep it).
fn explicit_null<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    return Option::<T>::deserialize(deserializer).map(Some);
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub tax_identification_number: Option<String>,
    pub customer_group_id: Option<Uuid>,
    #[serde(default)]
    pub marketing_consent: bool,
}

/// Every field optional - only the ones present change. Optional contact fields can be cleared with an explicit `null`.
#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub full_name: Option<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub address: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub tax_identification_number: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub customer_group_id: Option<Option<Uuid>>,
    pub marketing_consent: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListCustomersQuery {
    /// Matched against name, phone (raw and normalised) and email.
    #[serde(alias = "query")]
    pub q: Option<String>,
    /// The id of the last customer on the previous page.
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseHistoryEntry {
    pub sale_id: Uuid,
    pub receipt_number: Option<String>,
    pub store_id: Uuid,
    pub store_name: String,
    pub status: String,
    pub payment_status: String,
    pub total_amount_minor: i64,
    pub amount_refunded_minor: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones_and_emails_mask_to_a_recognisable_shape() {
        assert_eq!(mask_phone("+2348031234567"), "+234*******567");
        assert_eq!(mask_email("ada@example.com"), "a***@example.com");
    }
}
