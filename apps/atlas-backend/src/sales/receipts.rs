/*!
 * Receipts (FR-17). A receipt is numbered and snapshotted once, when its
 * sale completes; every view and reprint afterwards renders that snapshot,
 * never the live sale or catalogue, so historical receipts reproduce the
 * original transaction values exactly.
 */
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::Sale;
use super::service;

pub const SNAPSHOT_VERSION: i16 = 1;

// === Snapshot

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptBusiness {
    pub name: String,
    pub legal_name: Option<String>,
    pub tax_identification_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReceiptStore {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
    pub currency: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptLine {
    pub line_number: i32,
    pub name: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    pub unit_name: String,
    pub unit_price_amount_minor: i64,
    /// Unit price times quantity, before discounts and before any tax added on top.
    pub gross_amount_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_rate_basis_points: i32,
    pub tax_amount_minor: i64,
    pub line_total_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdown {
    pub rate_basis_points: i32,
    pub taxable_amount_minor: i64,
    pub tax_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPayment {
    pub method_name: String,
    pub kind: String,
    pub amount_minor: i64,
    pub tendered_amount_minor: Option<i64>,
    pub change_amount_minor: i64,
    /// Last four characters only - enough to match a bank statement, not enough to reuse.
    pub reference: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSnapshot {
    pub version: i16,
    pub business: ReceiptBusiness,
    pub store: ReceiptStore,
    pub transaction_id: Uuid,
    /// The last 8 hex digits of the transaction id - what an offline till prints before a receipt number exists.
    pub short_transaction_id: String,
    pub receipt_number: String,
    pub issued_at: DateTime<Utc>,
    /// `issued_at` in the store's own timezone, as printed.
    pub local_time: String,
    pub cashier_name: String,
    pub customer_name: String,
    pub device_id: String,
    pub prices_include_tax: bool,
    pub lines: Vec<ReceiptLine>,
    pub subtotal_amount_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub tax_breakdown: Vec<TaxBreakdown>,
    pub rounding_adjustment_minor: i64,
    pub total_amount_minor: i64,
    pub amount_due_minor: i64,
    pub payments: Vec<ReceiptPayment>,
    pub amount_paid_minor: i64,
    pub balance_minor: i64,
}

pub fn short_id(id: Uuid) -> String {
    let simple = id.simple().to_string();
    return simple[simple.len() - 8..].to_uppercase();
}

fn mask_reference(reference: &str) -> String {
    let chars: Vec<char> = reference.chars().collect();
    if chars.len() <= 4 {
        return reference.to_string();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    return format!("****{}", tail);
}

fn local_time(at: DateTime<Utc>, timezone: &str) -> String {
    return match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => at.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string(),
        Err(_) => at.format("%Y-%m-%d %H:%M UTC").to_string(),
    };
}

// === Numbering and snapshot

/**
 * The next receipt number for a store, from its counter row under that row's lock (the second lock in the documented order, after the sale row).
 * Formatted `{prefix}-{seq:08}`, see `store_prefix`.
 */
async fn next_receipt_number(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid) -> Result<String, ApiError> {
    sqlx::query(
        "INSERT INTO atlas.store_receipt_sequences (store_id, tenant_id) VALUES ($1, $2) ON CONFLICT (store_id) DO NOTHING",
    )
    .bind(store_id)
    .bind(tenant_id)
    .execute(&mut *conn)
    .await?;
    let sequence: i64 = sqlx::query_scalar(
        "UPDATE atlas.store_receipt_sequences SET next_value = next_value + 1 WHERE store_id = $1 RETURNING next_value - 1",
    )
    .bind(store_id)
    .fetch_one(&mut *conn)
    .await?;
    let prefix = store_prefix(conn, store_id).await?;
    return Ok(format!("{}-{:08}", prefix, sequence));
}

/// The store's `receipt_prefix`, else the first four letters or digits of its slug, upper-cased. Shared by receipt and return numbers.
pub async fn store_prefix(conn: &mut PgConnection, store_id: Uuid) -> Result<String, ApiError> {
    return sqlx::query_scalar(
        "SELECT COALESCE(NULLIF(trim(receipt_prefix), ''), upper(left(regexp_replace(slug, '[^a-zA-Z0-9]', '', 'g'), 4)))
         FROM atlas.stores WHERE id = $1",
    )
    .bind(store_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

/// Numbers a just-completed sale and writes its immutable receipt snapshot. Called once, from `checkout::complete_sale`.
pub async fn issue(conn: &mut PgConnection, tenant_id: Uuid, sale_id: Uuid) -> Result<(), ApiError> {
    let sale = service::get_sale(conn, sale_id).await?;
    let receipt_number = next_receipt_number(conn, tenant_id, sale.store_id).await?;
    sqlx::query("UPDATE atlas.sales SET receipt_number = $2 WHERE id = $1")
        .bind(sale_id)
        .bind(&receipt_number)
        .execute(&mut *conn)
        .await?;

    let issued_at = Utc::now();
    let snapshot = build_snapshot(conn, &sale, &receipt_number, issued_at).await?;
    sqlx::query(
        "INSERT INTO atlas.sale_receipts (id, tenant_id, sale_id, receipt_number, issued_at, snapshot, snapshot_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(sale_id)
    .bind(&receipt_number)
    .bind(issued_at)
    .bind(serde_json::to_value(&snapshot).map_err(|e| ApiError::Internal(e.to_string()))?)
    .bind(SNAPSHOT_VERSION)
    .execute(&mut *conn)
    .await?;
    return Ok(());
}

async fn build_snapshot(
    conn: &mut PgConnection,
    sale: &Sale,
    receipt_number: &str,
    issued_at: DateTime<Utc>,
) -> Result<ReceiptSnapshot, ApiError> {
    let business: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT t.name, t.legal_name, t.tax_identification_number
         FROM atlas.tenants t JOIN atlas.stores s ON s.tenant_id = t.id WHERE s.id = $1",
    )
    .bind(sale.store_id)
    .fetch_one(&mut *conn)
    .await?;
    let store = sqlx::query_as::<_, ReceiptStore>(
        "SELECT id, name, address, phone, receipt_header, receipt_footer, currency, timezone FROM atlas.stores WHERE id = $1",
    )
    .bind(sale.store_id)
    .fetch_one(&mut *conn)
    .await?;
    let cashier_name: String = sqlx::query_scalar("SELECT full_name FROM atlas.users WHERE id = $1")
        .bind(sale.cashier_user_id)
        .fetch_one(&mut *conn)
        .await?;
    let customer_name = match sale.customer_id {
        Some(customer_id) => sqlx::query_scalar("SELECT full_name FROM atlas.customers WHERE id = $1")
            .bind(customer_id)
            .fetch_one(&mut *conn)
            .await?,
        None => "Walk-in customer".to_string(),
    };

    let lines = service::list_sale_lines(conn, sale.id).await?;
    let payments = crate::payments::service::list_payments_for_sale(conn, sale.id).await?;

    let mut breakdown: BTreeMap<i32, (i64, i64)> = BTreeMap::new();
    for line in lines.iter().filter(|l| l.tax_rate_basis_points > 0) {
        let entry = breakdown.entry(line.tax_rate_basis_points).or_default();
        entry.0 += line.net_amount_minor;
        entry.1 += line.tax_amount_minor;
    }

    let due = sale.total_amount_minor + sale.rounding_adjustment_minor;
    return Ok(ReceiptSnapshot {
        version: SNAPSHOT_VERSION,
        business: ReceiptBusiness {
            name: business.0,
            legal_name: business.1,
            tax_identification_number: business.2,
        },
        local_time: local_time(issued_at, &store.timezone),
        store,
        transaction_id: sale.id,
        short_transaction_id: short_id(sale.id),
        receipt_number: receipt_number.to_string(),
        issued_at,
        cashier_name,
        customer_name,
        device_id: sale.device_id.clone(),
        prices_include_tax: sale.prices_include_tax,
        lines: lines
            .iter()
            .map(|line| ReceiptLine {
                line_number: line.line_number,
                name: line.product_name.clone(),
                sku: line.sku.clone(),
                quantity: line.quantity,
                unit_name: line.unit_name.clone(),
                unit_price_amount_minor: line.unit_price_amount_minor,
                gross_amount_minor: line.net_amount_minor + line.line_discount_amount_minor + line.allocated_cart_discount_amount_minor,
                discount_amount_minor: line.line_discount_amount_minor + line.allocated_cart_discount_amount_minor,
                tax_rate_basis_points: line.tax_rate_basis_points,
                tax_amount_minor: line.tax_amount_minor,
                line_total_amount_minor: line.line_total_amount_minor,
            })
            .collect(),
        subtotal_amount_minor: sale.subtotal_amount_minor,
        discount_amount_minor: sale.discount_amount_minor,
        tax_amount_minor: sale.tax_amount_minor,
        tax_breakdown: breakdown
            .into_iter()
            .map(|(rate, (taxable, tax))| TaxBreakdown {
                rate_basis_points: rate,
                taxable_amount_minor: taxable,
                tax_amount_minor: tax,
            })
            .collect(),
        rounding_adjustment_minor: sale.rounding_adjustment_minor,
        total_amount_minor: sale.total_amount_minor,
        amount_due_minor: due,
        payments: payments
            .iter()
            .filter(|p| p.status == "succeeded" || p.status == "pending")
            .map(|p| ReceiptPayment {
                method_name: p.method_name.clone(),
                kind: p.method.clone(),
                amount_minor: p.amount_minor,
                tendered_amount_minor: p.tendered_amount_minor,
                change_amount_minor: p.change_amount_minor,
                reference: p.reference.as_deref().map(mask_reference),
                status: p.status.clone(),
            })
            .collect(),
        amount_paid_minor: sale.amount_paid_minor,
        balance_minor: (due - sale.amount_paid_minor).max(0),
    });
}

// === Reading and printing

/// Where the sale stands now - separate from the snapshot, which never changes after issue.
#[derive(Debug, Serialize)]
pub struct CurrentState {
    pub status: String,
    pub payment_status: String,
    pub amount_paid_minor: i64,
    pub amount_refunded_minor: i64,
    pub balance_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct ReceiptView {
    pub sale_id: Uuid,
    pub receipt_number: String,
    pub issued_at: DateTime<Utc>,
    pub snapshot: ReceiptSnapshot,
    pub current: CurrentState,
    pub print_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PrintedReceipt {
    pub kind: String,
    pub copy_number: i32,
    pub text: String,
    pub receipt: ReceiptView,
}

pub async fn view(conn: &mut PgConnection, sale_id: Uuid) -> Result<ReceiptView, ApiError> {
    let row: Option<(Uuid, String, DateTime<Utc>, serde_json::Value)> = sqlx::query_as(
        "SELECT id, receipt_number, issued_at, snapshot FROM atlas.sale_receipts WHERE sale_id = $1",
    )
    .bind(sale_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((receipt_id, receipt_number, issued_at, snapshot)) = row else {
        return Err(ApiError::not_found("This sale has no receipt yet - it isn't completed."));
    };
    let snapshot: ReceiptSnapshot = serde_json::from_value(snapshot).map_err(|e| ApiError::Internal(e.to_string()))?;
    let sale = service::get_sale(conn, sale_id).await?;
    let print_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.receipt_prints WHERE sale_receipt_id = $1")
        .bind(receipt_id)
        .fetch_one(&mut *conn)
        .await?;
    let due = sale.total_amount_minor + sale.rounding_adjustment_minor;
    return Ok(ReceiptView {
        sale_id,
        receipt_number,
        issued_at,
        snapshot,
        current: CurrentState {
            balance_minor: (due - sale.amount_paid_minor).max(0),
            status: sale.status,
            payment_status: sale.payment_status,
            amount_paid_minor: sale.amount_paid_minor,
            amount_refunded_minor: sale.amount_refunded_minor,
        },
        print_count,
    });
}

/**
 * Records a print and returns the text to send to the printer. The first print is the `original`; every one after it is a `reprint` with an increasing copy number, and the text says so.
 * The receipt row is locked so two tills reprinting at once still get distinct copy numbers.
 */
pub async fn record_print(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    sale_id: Uuid,
    printed_by_user_id: Uuid,
    device_id: Option<&str>,
    reason: Option<&str>,
    width: usize,
) -> Result<PrintedReceipt, ApiError> {
    let receipt_id: Uuid = sqlx::query_scalar("SELECT id FROM atlas.sale_receipts WHERE sale_id = $1 FOR UPDATE")
        .bind(sale_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::not_found("This sale has no receipt yet - it isn't completed."))?;
    let copy_number: i32 = sqlx::query_scalar("SELECT COUNT(*)::int + 1 FROM atlas.receipt_prints WHERE sale_receipt_id = $1")
        .bind(receipt_id)
        .fetch_one(&mut *conn)
        .await?;
    let kind = if copy_number == 1 { "original" } else { "reprint" };
    sqlx::query(
        "INSERT INTO atlas.receipt_prints (id, tenant_id, sale_receipt_id, printed_by_user_id, device_id, kind, copy_number, reason)
         VALUES ($1, $2, $3, $4, $5, $6::atlas.receipt_print_kind, $7, $8)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(receipt_id)
    .bind(printed_by_user_id)
    .bind(device_id)
    .bind(kind)
    .bind(copy_number)
    .bind(reason)
    .execute(&mut *conn)
    .await?;

    let receipt = view(conn, sale_id).await?;
    let text = render_text(&receipt.snapshot, width, if copy_number > 1 { Some(copy_number) } else { None });
    return Ok(PrintedReceipt {
        kind: kind.to_string(),
        copy_number,
        text,
        receipt,
    });
}

/// How many prints a receipt has had so far, for deciding whether the next one is a reprint.
pub async fn print_count(conn: &mut PgConnection, sale_id: Uuid) -> Result<i64, ApiError> {
    return sqlx::query_scalar(
        "SELECT COUNT(rp.id) FROM atlas.sale_receipts sr
         LEFT JOIN atlas.receipt_prints rp ON rp.sale_receipt_id = sr.id
         WHERE sr.sale_id = $1",
    )
    .bind(sale_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

// === Plain-text rendering

fn money(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let abs = amount_minor.unsigned_abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    return format!("{}{}.{:02}", sign, grouped, abs % 100);
}

fn row(left: &str, right: &str, width: usize) -> String {
    let left: String = left.chars().take(width.saturating_sub(right.chars().count() + 1)).collect();
    let gap = width.saturating_sub(left.chars().count() + right.chars().count()).max(1);
    return format!("{}{}{}", left, " ".repeat(gap), right);
}

fn centered(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        return text.chars().take(width).collect();
    }
    return format!("{}{}", " ".repeat((width - len) / 2), text);
}

/**
 * A fixed-width rendering for 58mm (32 columns) and 80mm (48 columns) thermal printers. The till sends it to the printer as-is; amounts are in major units of the store currency, assuming two minor digits (kobo, cents).
 * `reprint_copy` adds the reprint banner, so a reprint can never pass for the original.
 */
pub fn render_text(snapshot: &ReceiptSnapshot, width: usize, reprint_copy: Option<i32>) -> String {
    let width = width.clamp(24, 64);
    let rule = "-".repeat(width);
    let mut out: Vec<String> = Vec::new();

    if let Some(copy) = reprint_copy {
        out.push(centered(&format!("*** REPRINT (copy {}) ***", copy), width));
    }
    out.push(centered(snapshot.business.legal_name.as_deref().unwrap_or(&snapshot.business.name), width));
    out.push(centered(&snapshot.store.name, width));
    for extra in [&snapshot.store.address, &snapshot.store.phone, &snapshot.store.receipt_header].into_iter().flatten() {
        out.push(centered(extra, width));
    }
    if let Some(tin) = &snapshot.business.tax_identification_number {
        out.push(centered(&format!("TIN: {}", tin), width));
    }
    out.push(rule.clone());
    out.push(row("Receipt", &snapshot.receipt_number, width));
    out.push(row("Txn", &snapshot.short_transaction_id, width));
    out.push(row("Date", &snapshot.local_time, width));
    out.push(row("Cashier", &snapshot.cashier_name, width));
    out.push(row("Customer", &snapshot.customer_name, width));
    out.push(rule.clone());

    for line in &snapshot.lines {
        out.push(line.name.chars().take(width).collect());
        let detail = format!("  {} {} x {}", line.quantity.normalize(), line.unit_name, money(line.unit_price_amount_minor));
        out.push(row(&detail, &money(line.gross_amount_minor), width));
        if line.discount_amount_minor > 0 {
            out.push(row("  Discount", &money(-line.discount_amount_minor), width));
        }
    }
    out.push(rule.clone());
    out.push(row("Subtotal", &money(snapshot.subtotal_amount_minor), width));
    if snapshot.discount_amount_minor > 0 {
        out.push(row("Discount", &money(-snapshot.discount_amount_minor), width));
    }
    for tax in &snapshot.tax_breakdown {
        let label = format!(
            "Tax {}%{}",
            Decimal::new(i64::from(tax.rate_basis_points), 2).normalize(),
            if snapshot.prices_include_tax { " (incl.)" } else { "" }
        );
        out.push(row(&label, &money(tax.tax_amount_minor), width));
    }
    if snapshot.rounding_adjustment_minor != 0 {
        out.push(row("Rounding", &money(snapshot.rounding_adjustment_minor), width));
    }
    out.push(row(&format!("TOTAL ({})", snapshot.store.currency), &money(snapshot.amount_due_minor), width));
    out.push(rule.clone());
    for payment in &snapshot.payments {
        let label = match &payment.reference {
            Some(reference) => format!("{} {}", payment.method_name, reference),
            None => payment.method_name.clone(),
        };
        let paid = payment.tendered_amount_minor.unwrap_or(payment.amount_minor);
        out.push(row(&label, &money(paid), width));
        if payment.change_amount_minor > 0 {
            out.push(row("  Change", &money(payment.change_amount_minor), width));
        }
    }
    out.push(row("Paid", &money(snapshot.amount_paid_minor), width));
    if snapshot.balance_minor > 0 {
        out.push(row("Balance due", &money(snapshot.balance_minor), width));
    }
    if let Some(footer) = &snapshot.store.receipt_footer {
        out.push(rule.clone());
        out.push(centered(footer, width));
    }
    if let Some(copy) = reprint_copy {
        out.push(centered(&format!("*** REPRINT (copy {}) ***", copy), width));
    }
    return out.join("\n") + "\n";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_thousands_and_keeps_two_decimals() {
        assert_eq!(money(15_000_000), "150,000.00");
        assert_eq!(money(5), "0.05");
        assert_eq!(money(-123_456), "-1,234.56");
    }

    #[test]
    fn rows_fit_the_paper_width() {
        let line = row("A very long product name that will not fit", "1,000.00", 32);
        assert_eq!(line.chars().count(), 32);
        assert!(line.ends_with("1,000.00"));
    }

    #[test]
    fn references_keep_only_their_last_four_characters() {
        assert_eq!(mask_reference("TRF-00012345"), "****2345");
        assert_eq!(mask_reference("123"), "123");
    }
}
