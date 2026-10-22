/*!
 * The slip handed to a customer when a return completes. Everything on it
 * is already immutable - the original sale's receipt snapshot for the
 * business and store details, and the return's own lines and refunds,
 * which never change once completed - so it's rendered on demand rather
 * than stored as a second snapshot.
 */
use crate::sales::receipts::ReceiptSnapshot;

use super::models::ReturnDetail;

fn money(amount_minor: i64) -> String {
    let abs = amount_minor.unsigned_abs();
    return format!("{}{}.{:02}", if amount_minor < 0 { "-" } else { "" }, abs / 100, abs % 100);
}

fn row(left: &str, right: &str, width: usize) -> String {
    let left: String = left.chars().take(width.saturating_sub(right.chars().count() + 1)).collect();
    let gap = width.saturating_sub(left.chars().count() + right.chars().count()).max(1);
    return format!("{}{}{}", left, " ".repeat(gap), right);
}

pub fn render_text(sale: &ReceiptSnapshot, detail: &ReturnDetail, width: usize) -> String {
    let width = width.clamp(24, 64);
    let rule = "-".repeat(width);
    let mut out = vec![
        sale.business.legal_name.clone().unwrap_or_else(|| sale.business.name.clone()),
        sale.store.name.clone(),
        "RETURN / REFUND".to_string(),
        rule.clone(),
        row("Return", detail.header.return_number.as_deref().unwrap_or("pending"), width),
        row("Original receipt", &sale.receipt_number, width),
        row("Reason", &detail.header.reason_code, width),
        rule.clone(),
    ];
    for line in &detail.lines {
        out.push(line.product_name.chars().take(width).collect());
        let label = format!("  {} {} ({})", line.quantity.normalize(), line.unit_name, line.condition);
        out.push(row(&label, &money(line.refund_amount_minor), width));
    }
    out.push(rule.clone());
    out.push(row("Refund total", &money(detail.header.refund_total_minor), width));
    for refund in &detail.refunds {
        out.push(row(refund.method_name.as_deref().unwrap_or("Refund"), &money(refund.amount_minor), width));
    }
    return out.join("\n") + "\n";
}
