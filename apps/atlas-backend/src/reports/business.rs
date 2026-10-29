/*!
 * FR-25 operational reports. Each is limited to the stores where the caller holds `report.view`; naming any
 * other store is a 403. Revenue, collected payments, refunds, expenses, inventory value and profit are kept as
 * separate figures, never folded into one another. Every report also downloads as CSV (`format=csv`) or PDF (`format=pdf`) (FR-28).
 *
 * Definitions: revenue counts completed sales, by when they completed. Net revenue is revenue less tax. Cost of
 * goods uses the unit cost each line was sold at. Collected is succeeded payments by when they were taken;
 * refunds are executed refunds by when they ran.
 */
use std::collections::BTreeMap;

use axum::extract::Query;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles::{self, service::StoreScope};

// === Filters and output

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub store_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub supplier_id: Option<Uuid>,
    pub cashier_user_id: Option<Uuid>,
    /// Stock movements only: `sale`, `adjustment`, `transfer_out`, ...
    pub movement_type: Option<String>,
    /// Stock movements only: the sale, transfer, order or count that caused them.
    pub reference_id: Option<Uuid>,
    /// `csv` or `pdf` for a download; JSON otherwise.
    pub format: Option<String>,
}

/// A report row that can also be written as one CSV line.
pub trait CsvRow {
    fn header() -> &'static [&'static str];
    fn cells(&self) -> Vec<String>;
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        return format!("\"{}\"", value.replace('"', "\"\""));
    }
    return value.to_string();
}

pub fn to_csv<R: CsvRow>(rows: &[R]) -> String {
    let mut out = String::new();
    out.push_str(&R::header().join(","));
    out.push('\n');
    for row in rows {
        let cells: Vec<String> = row.cells().iter().map(|c| csv_escape(c)).collect();
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    return out;
}

fn opt<T: ToString>(value: &Option<T>) -> String {
    return value.as_ref().map(ToString::to_string).unwrap_or_default();
}

fn respond<R: CsvRow + Serialize>(query: &ReportQuery, name: &str, message: &str, rows: Vec<R>) -> Response {
    if query.format.as_deref() == Some("pdf") {
        let cells: Vec<Vec<String>> = rows.iter().map(CsvRow::cells).collect();
        return (
            [
                (header::CONTENT_TYPE, "application/pdf".to_string()),
                (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.pdf\"", name)),
            ],
            crate::pdf::table(message.trim_end_matches(" generated."), R::header(), &cells),
        )
            .into_response();
    }
    if query.format.as_deref() == Some("csv") {
        return (
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.csv\"", name)),
            ],
            to_csv(&rows),
        )
            .into_response();
    }
    return Json(ApiEnvelope::ok(message, rows)).into_response();
}

/// The stores this report may cover: the one asked for (if the caller may see it) or every store they may see.
async fn report_stores(tx: &mut TenantTx, store_id: Option<Uuid>) -> Result<Vec<Uuid>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let scope = roles::service::stores_with_permission(tx, user_id, tenant_id, "report.view").await?;
    let visible: Vec<Uuid> = match scope {
        StoreScope::All => sqlx::query_scalar("SELECT id FROM atlas.stores WHERE tenant_id = $1 AND deleted_at IS NULL ORDER BY created_at")
            .bind(tenant_id)
            .fetch_all(&mut **tx)
            .await?,
        StoreScope::Stores(stores) => stores,
    };
    if visible.is_empty() {
        return Err(ApiError::forbidden("Missing permission: report.view"));
    }
    return match store_id {
        Some(id) if visible.contains(&id) => Ok(vec![id]),
        Some(_) => Err(ApiError::forbidden("Missing permission: report.view")),
        None => Ok(visible),
    };
}

// === Summary

#[derive(Debug, Default, Clone, Serialize)]
pub struct Metrics {
    pub sales_count: i64,
    pub revenue_minor: i64,
    pub tax_minor: i64,
    pub discounts_minor: i64,
    pub net_revenue_minor: i64,
    pub cost_of_goods_minor: i64,
    pub gross_profit_minor: i64,
    pub payments_collected_minor: i64,
    pub refunds_minor: i64,
    pub returns_count: i64,
    pub expenses_minor: i64,
    pub purchases_received_minor: i64,
    pub transfers_out_count: i64,
    pub transfers_in_count: i64,
    /// On hand right now at each line's listed cost; not limited by the date range.
    pub inventory_value_minor: i64,
}

impl Metrics {
    fn add(&mut self, other: &Metrics) {
        self.sales_count += other.sales_count;
        self.revenue_minor += other.revenue_minor;
        self.tax_minor += other.tax_minor;
        self.discounts_minor += other.discounts_minor;
        self.net_revenue_minor += other.net_revenue_minor;
        self.cost_of_goods_minor += other.cost_of_goods_minor;
        self.gross_profit_minor += other.gross_profit_minor;
        self.payments_collected_minor += other.payments_collected_minor;
        self.refunds_minor += other.refunds_minor;
        self.returns_count += other.returns_count;
        self.expenses_minor += other.expenses_minor;
        self.purchases_received_minor += other.purchases_received_minor;
        self.transfers_out_count += other.transfers_out_count;
        self.transfers_in_count += other.transfers_in_count;
        self.inventory_value_minor += other.inventory_value_minor;
    }
}

#[derive(Debug, Serialize)]
pub struct StoreMetrics {
    pub store_id: Uuid,
    pub store_name: String,
    #[serde(flatten)]
    pub metrics: Metrics,
}

impl CsvRow for StoreMetrics {
    fn header() -> &'static [&'static str] {
        return &[
            "store_id", "store_name", "sales_count", "revenue_minor", "tax_minor", "discounts_minor", "net_revenue_minor",
            "cost_of_goods_minor", "gross_profit_minor", "payments_collected_minor", "refunds_minor", "returns_count",
            "expenses_minor", "purchases_received_minor", "transfers_out_count", "transfers_in_count", "inventory_value_minor",
        ];
    }

    fn cells(&self) -> Vec<String> {
        let m = &self.metrics;
        return vec![
            self.store_id.to_string(), self.store_name.clone(), m.sales_count.to_string(), m.revenue_minor.to_string(),
            m.tax_minor.to_string(), m.discounts_minor.to_string(), m.net_revenue_minor.to_string(),
            m.cost_of_goods_minor.to_string(), m.gross_profit_minor.to_string(), m.payments_collected_minor.to_string(),
            m.refunds_minor.to_string(), m.returns_count.to_string(), m.expenses_minor.to_string(),
            m.purchases_received_minor.to_string(), m.transfers_out_count.to_string(), m.transfers_in_count.to_string(),
            m.inventory_value_minor.to_string(),
        ];
    }
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub totals: Metrics,
    pub stores: Vec<StoreMetrics>,
}

/// One `(store_id, value)` aggregate over the report's stores and window.
async fn per_store(tx: &mut TenantTx, sql: &str, stores: &[Uuid], query: &ReportQuery) -> Result<BTreeMap<Uuid, i64>, ApiError> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(sql)
        .bind(stores)
        .bind(query.from)
        .bind(query.to)
        .fetch_all(&mut **tx)
        .await?;
    return Ok(rows.into_iter().collect());
}

/// `GET /api/v1/reports/summary?from=&to=&store_id=`. One row per store plus the totals across them.
pub async fn summary(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    const WINDOW: &str = "($2::timestamptz IS NULL OR {at} >= $2) AND ($3::timestamptz IS NULL OR {at} < $3)";
    let window = |at: &str| WINDOW.replace("{at}", at);

    let sales_sql = format!(
        "SELECT store_id, COUNT(*)::bigint, COALESCE(SUM(total_amount_minor), 0)::bigint, COALESCE(SUM(tax_amount_minor), 0)::bigint,
                COALESCE(SUM(discount_amount_minor + cart_discount_amount_minor), 0)::bigint
         FROM atlas.sales WHERE store_id = ANY($1) AND status = 'completed' AND {}
         GROUP BY store_id",
        window("completed_at")
    );
    let sales: Vec<(Uuid, i64, i64, i64, i64)> = sqlx::query_as(&sales_sql)
        .bind(&stores)
        .bind(query.from)
        .bind(query.to)
        .fetch_all(&mut *tx)
        .await?;

    let cost = per_store(&mut tx, &format!(
        "SELECT s.store_id, COALESCE(SUM(ROUND(l.base_quantity * l.unit_cost_amount_minor)), 0)::bigint
         FROM atlas.sale_lines l JOIN atlas.sales s ON s.id = l.sale_id
         WHERE s.store_id = ANY($1) AND s.status = 'completed' AND {} GROUP BY s.store_id",
        window("s.completed_at")
    ), &stores, &query).await?;
    let collected = per_store(&mut tx, &format!(
        "SELECT s.store_id, COALESCE(SUM(p.amount_minor), 0)::bigint
         FROM atlas.payments p JOIN atlas.sales s ON s.id = p.sale_id
         WHERE s.store_id = ANY($1) AND p.status IN ('succeeded', 'partially_refunded', 'refunded') AND {} GROUP BY s.store_id",
        window("p.created_at")
    ), &stores, &query).await?;
    let refunds = per_store(&mut tx, &format!(
        "SELECT s.store_id, COALESCE(SUM(r.amount_minor), 0)::bigint
         FROM atlas.refunds r JOIN atlas.sales s ON s.id = r.sale_id
         WHERE s.store_id = ANY($1) AND r.status = 'executed' AND {} GROUP BY s.store_id",
        window("r.created_at")
    ), &stores, &query).await?;
    let returns = per_store(&mut tx, &format!(
        "SELECT s.store_id, COUNT(*)::bigint FROM atlas.returns rt JOIN atlas.sales s ON s.id = rt.sale_id
         WHERE s.store_id = ANY($1) AND rt.status = 'completed' AND {} GROUP BY s.store_id",
        window("rt.completed_at")
    ), &stores, &query).await?;
    let expenses = per_store(&mut tx, &format!(
        "SELECT store_id, COALESCE(SUM(amount_minor), 0)::bigint FROM atlas.expenses
         WHERE store_id = ANY($1) AND status = 'approved' AND {} GROUP BY store_id",
        window("expense_date::timestamptz")
    ), &stores, &query).await?;
    let purchases = per_store(&mut tx, &format!(
        "SELECT po.store_id, (COALESCE(SUM(ROUND(gl.quantity_received * pol.unit_cost_amount_minor)), 0))::bigint
         FROM atlas.goods_received_note_lines gl
         JOIN atlas.goods_received_notes g ON g.id = gl.goods_received_note_id
         JOIN atlas.purchase_orders po ON po.id = g.purchase_order_id
         JOIN atlas.purchase_order_lines pol ON pol.id = gl.purchase_order_line_id
         WHERE po.store_id = ANY($1) AND {} GROUP BY po.store_id",
        window("g.received_at")
    ), &stores, &query).await?;
    let transfers_out = per_store(&mut tx, &format!(
        "SELECT from_store_id, COUNT(*)::bigint FROM atlas.stock_transfers
         WHERE from_store_id = ANY($1) AND status IN ('dispatched', 'received') AND {} GROUP BY from_store_id",
        window("dispatched_at")
    ), &stores, &query).await?;
    let transfers_in = per_store(&mut tx, &format!(
        "SELECT to_store_id, COUNT(*)::bigint FROM atlas.stock_transfers
         WHERE to_store_id = ANY($1) AND status = 'received' AND {} GROUP BY to_store_id",
        window("received_at")
    ), &stores, &query).await?;
    let inventory: BTreeMap<Uuid, i64> = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT m.store_id, COALESCE(SUM(ROUND(m.qty * COALESCE(l.cost_amount_minor, 0))), 0)::bigint
         FROM (SELECT store_id, product_id, SUM(quantity) AS qty FROM atlas.stock_movements
               WHERE store_id = ANY($1) GROUP BY store_id, product_id HAVING SUM(quantity) > 0) m
         LEFT JOIN atlas.product_store_listings l ON l.product_id = m.product_id AND l.store_id = m.store_id
         GROUP BY m.store_id",
    )
    .bind(&stores)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let names: BTreeMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM atlas.stores WHERE id = ANY($1)")
        .bind(&stores)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .collect();
    let sales_by_store: BTreeMap<Uuid, (i64, i64, i64, i64)> =
        sales.into_iter().map(|(id, count, revenue, tax, discounts)| (id, (count, revenue, tax, discounts))).collect();

    let get = |map: &BTreeMap<Uuid, i64>, id: &Uuid| -> i64 { map.get(id).copied().unwrap_or(0) };
    let mut totals = Metrics::default();
    let mut rows = Vec::with_capacity(stores.len());
    for store_id in &stores {
        let (count, revenue, tax, discounts) = sales_by_store.get(store_id).copied().unwrap_or((0, 0, 0, 0));
        let net = revenue - tax;
        let cogs = get(&cost, store_id);
        let metrics = Metrics {
            sales_count: count,
            revenue_minor: revenue,
            tax_minor: tax,
            discounts_minor: discounts,
            net_revenue_minor: net,
            cost_of_goods_minor: cogs,
            gross_profit_minor: net - cogs,
            payments_collected_minor: get(&collected, store_id),
            refunds_minor: get(&refunds, store_id),
            returns_count: get(&returns, store_id),
            expenses_minor: get(&expenses, store_id),
            purchases_received_minor: get(&purchases, store_id),
            transfers_out_count: get(&transfers_out, store_id),
            transfers_in_count: get(&transfers_in, store_id),
            inventory_value_minor: get(&inventory, store_id),
        };
        totals.add(&metrics);
        rows.push(StoreMetrics { store_id: *store_id, store_name: names.get(store_id).cloned().unwrap_or_default(), metrics });
    }
    tx.commit().await?;
    if matches!(query.format.as_deref(), Some("csv" | "pdf")) {
        return Ok(respond(&query, "summary", "Summary generated.", rows));
    }
    return Ok(Json(ApiEnvelope::ok("Summary generated.", Summary { from: query.from, to: query.to, totals, stores: rows })).into_response());
}

// === Sales by product

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductSales {
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    pub net_amount_minor: i64,
    pub tax_minor: i64,
    pub cost_of_goods_minor: i64,
    pub gross_profit_minor: i64,
}

impl CsvRow for ProductSales {
    fn header() -> &'static [&'static str] {
        return &["product_id", "product_name", "sku", "quantity", "net_amount_minor", "tax_minor", "cost_of_goods_minor", "gross_profit_minor"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.product_id.to_string(), self.product_name.clone(), opt(&self.sku), self.quantity.normalize().to_string(),
            self.net_amount_minor.to_string(), self.tax_minor.to_string(), self.cost_of_goods_minor.to_string(),
            self.gross_profit_minor.to_string(),
        ];
    }
}

/// `GET /api/v1/reports/sales-by-product?from=&to=&store_id=&product_id=&category_id=&cashier_user_id=&format=`. Best sellers first.
pub async fn sales_by_product(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, ProductSales>(
        "SELECT l.product_id, MAX(l.product_name) AS product_name, MAX(l.sku) AS sku,
                COALESCE(SUM(l.base_quantity), 0) AS quantity,
                COALESCE(SUM(l.net_amount_minor), 0)::bigint AS net_amount_minor,
                COALESCE(SUM(l.tax_amount_minor), 0)::bigint AS tax_minor,
                COALESCE(SUM(ROUND(l.base_quantity * l.unit_cost_amount_minor)), 0)::bigint AS cost_of_goods_minor,
                (COALESCE(SUM(l.net_amount_minor), 0) - COALESCE(SUM(ROUND(l.base_quantity * l.unit_cost_amount_minor)), 0))::bigint AS gross_profit_minor
         FROM atlas.sale_lines l
         JOIN atlas.sales s ON s.id = l.sale_id
         JOIN atlas.products p ON p.id = l.product_id
         WHERE s.store_id = ANY($1) AND s.status = 'completed'
           AND ($2::timestamptz IS NULL OR s.completed_at >= $2) AND ($3::timestamptz IS NULL OR s.completed_at < $3)
           AND ($4::uuid IS NULL OR l.product_id = $4)
           AND ($5::uuid IS NULL OR p.category_id = $5)
           AND ($6::uuid IS NULL OR s.cashier_user_id = $6)
         GROUP BY l.product_id
         ORDER BY net_amount_minor DESC, product_name",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.product_id)
    .bind(query.category_id)
    .bind(query.cashier_user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "sales-by-product", "Sales by product generated.", rows));
}

// === Inventory valuation

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryValue {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub on_hand: Decimal,
    pub unit_cost_amount_minor: Option<i64>,
    pub value_minor: i64,
}

impl CsvRow for InventoryValue {
    fn header() -> &'static [&'static str] {
        return &["store_id", "product_id", "product_name", "on_hand", "unit_cost_amount_minor", "value_minor"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.store_id.to_string(), self.product_id.to_string(), self.product_name.clone(), self.on_hand.normalize().to_string(),
            opt(&self.unit_cost_amount_minor), self.value_minor.to_string(),
        ];
    }
}

/// `GET /api/v1/reports/inventory-valuation?store_id=&category_id=&product_id=&format=`. Current stock at listed cost.
pub async fn inventory_valuation(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, InventoryValue>(
        "SELECT m.store_id, m.product_id, p.name AS product_name, SUM(m.quantity) AS on_hand,
                MAX(l.cost_amount_minor) AS unit_cost_amount_minor,
                ROUND(SUM(m.quantity) * COALESCE(MAX(l.cost_amount_minor), 0))::bigint AS value_minor
         FROM atlas.stock_movements m
         JOIN atlas.products p ON p.id = m.product_id
         LEFT JOIN atlas.product_store_listings l ON l.product_id = m.product_id AND l.store_id = m.store_id
         WHERE m.store_id = ANY($1)
           AND ($2::uuid IS NULL OR m.product_id = $2)
           AND ($3::uuid IS NULL OR p.category_id = $3)
         GROUP BY m.store_id, m.product_id, p.name
         HAVING SUM(m.quantity) <> 0
         ORDER BY value_minor DESC, p.name",
    )
    .bind(&stores)
    .bind(query.product_id)
    .bind(query.category_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "inventory-valuation", "Inventory valuation generated.", rows));
}

// === Purchases by supplier

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SupplierPurchases {
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub orders_count: i64,
    pub received_minor: i64,
    pub returned_minor: i64,
    pub net_purchases_minor: i64,
}

impl CsvRow for SupplierPurchases {
    fn header() -> &'static [&'static str] {
        return &["supplier_id", "supplier_name", "orders_count", "received_minor", "returned_minor", "net_purchases_minor"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.supplier_id.to_string(), self.supplier_name.clone(), self.orders_count.to_string(), self.received_minor.to_string(),
            self.returned_minor.to_string(), self.net_purchases_minor.to_string(),
        ];
    }
}

/// `GET /api/v1/reports/purchases-by-supplier?from=&to=&store_id=&supplier_id=&format=`. Receipts and confirmed returns in the window.
pub async fn purchases_by_supplier(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, SupplierPurchases>(
        "WITH received AS (
             SELECT po.supplier_id, COUNT(DISTINCT po.id)::bigint AS orders, SUM(ROUND(gl.quantity_received * pol.unit_cost_amount_minor)) AS amount
             FROM atlas.goods_received_note_lines gl
             JOIN atlas.goods_received_notes g ON g.id = gl.goods_received_note_id
             JOIN atlas.purchase_orders po ON po.id = g.purchase_order_id
             JOIN atlas.purchase_order_lines pol ON pol.id = gl.purchase_order_line_id
             WHERE po.store_id = ANY($1)
               AND ($2::timestamptz IS NULL OR g.received_at >= $2) AND ($3::timestamptz IS NULL OR g.received_at < $3)
             GROUP BY po.supplier_id
         ), returned AS (
             SELECT r.supplier_id, SUM(ROUND(rl.quantity * rl.unit_cost_amount_minor)) AS amount
             FROM atlas.purchase_return_lines rl JOIN atlas.purchase_returns r ON r.id = rl.purchase_return_id
             WHERE r.store_id = ANY($1) AND r.status = 'confirmed'
               AND ($2::timestamptz IS NULL OR r.confirmed_at >= $2) AND ($3::timestamptz IS NULL OR r.confirmed_at < $3)
             GROUP BY r.supplier_id
         )
         SELECT s.id AS supplier_id, s.name AS supplier_name, COALESCE(rc.orders, 0)::bigint AS orders_count,
                COALESCE(rc.amount, 0)::bigint AS received_minor, COALESCE(rt.amount, 0)::bigint AS returned_minor,
                (COALESCE(rc.amount, 0) - COALESCE(rt.amount, 0))::bigint AS net_purchases_minor
         FROM atlas.suppliers s
         LEFT JOIN received rc ON rc.supplier_id = s.id
         LEFT JOIN returned rt ON rt.supplier_id = s.id
         WHERE (rc.supplier_id IS NOT NULL OR rt.supplier_id IS NOT NULL) AND ($4::uuid IS NULL OR s.id = $4)
         ORDER BY net_purchases_minor DESC, s.name",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.supplier_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "purchases-by-supplier", "Purchases by supplier generated.", rows));
}

// === Expenses by category

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryExpenses {
    pub store_id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub approved_minor: i64,
    pub paid_minor: i64,
    pub unpaid_minor: i64,
}

impl CsvRow for CategoryExpenses {
    fn header() -> &'static [&'static str] {
        return &["store_id", "category_id", "category_name", "approved_minor", "paid_minor", "unpaid_minor"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.store_id.to_string(), self.category_id.to_string(), self.category_name.clone(), self.approved_minor.to_string(),
            self.paid_minor.to_string(), self.unpaid_minor.to_string(),
        ];
    }
}

/// `GET /api/v1/reports/expenses-by-category?from=&to=&store_id=&category_id=&format=`. Approved expenses only, by expense date.
pub async fn expenses_by_category(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, CategoryExpenses>(
        "SELECT e.store_id, e.category_id, c.name AS category_name,
                SUM(e.amount_minor)::bigint AS approved_minor,
                SUM(CASE WHEN e.payment_status = 'paid' THEN e.amount_minor ELSE 0 END)::bigint AS paid_minor,
                SUM(CASE WHEN e.payment_status = 'unpaid' THEN e.amount_minor ELSE 0 END)::bigint AS unpaid_minor
         FROM atlas.expenses e JOIN atlas.expense_categories c ON c.id = e.category_id
         WHERE e.store_id = ANY($1) AND e.status = 'approved'
           AND ($2::timestamptz IS NULL OR e.expense_date::timestamptz >= $2) AND ($3::timestamptz IS NULL OR e.expense_date::timestamptz < $3)
           AND ($4::uuid IS NULL OR e.category_id = $4)
         GROUP BY e.store_id, e.category_id, c.name
         ORDER BY approved_minor DESC, c.name",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.category_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "expenses-by-category", "Expenses by category generated.", rows));
}

// === Stock movements

/// The most rows one movement report returns; narrow the filters for more.
const MOVEMENT_ROWS: i64 = 5_000;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MovementRow {
    pub created_at: DateTime<Utc>,
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub stock_item_id: Option<Uuid>,
    pub movement_type: String,
    pub quantity: Decimal,
    pub reason: String,
    pub reference_id: Option<Uuid>,
    pub created_by_user_id: Uuid,
}

impl CsvRow for MovementRow {
    fn header() -> &'static [&'static str] {
        return &["created_at", "store_id", "product_id", "product_name", "stock_item_id", "movement_type", "quantity", "reason", "reference_id", "created_by_user_id"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.created_at.to_rfc3339(), self.store_id.to_string(), self.product_id.to_string(), self.product_name.clone(),
            opt(&self.stock_item_id), self.movement_type.clone(), self.quantity.normalize().to_string(), self.reason.clone(),
            opt(&self.reference_id), self.created_by_user_id.to_string(),
        ];
    }
}

/// `GET /api/v1/reports/stock-movements?from=&to=&store_id=&product_id=&category_id=&movement_type=&reference_id=&format=`. Newest first.
pub async fn stock_movements(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, MovementRow>(
        "SELECT m.created_at, m.store_id, m.product_id, p.name AS product_name, m.stock_item_id, m.movement_type::text AS movement_type,
                m.quantity, m.reason, m.reference_id, m.created_by_user_id
         FROM atlas.stock_movements m JOIN atlas.products p ON p.id = m.product_id
         WHERE m.store_id = ANY($1)
           AND ($2::timestamptz IS NULL OR m.created_at >= $2) AND ($3::timestamptz IS NULL OR m.created_at < $3)
           AND ($4::uuid IS NULL OR m.product_id = $4)
           AND ($5::uuid IS NULL OR p.category_id = $5)
           AND ($6::text IS NULL OR m.movement_type::text = $6)
           AND ($7::uuid IS NULL OR m.reference_id = $7)
         ORDER BY m.created_at DESC, m.id DESC
         LIMIT $8",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.product_id)
    .bind(query.category_id)
    .bind(query.movement_type.as_deref())
    .bind(query.reference_id)
    .bind(MOVEMENT_ROWS)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "stock-movements", "Stock movements generated.", rows));
}

// === Transfers

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TransferRow {
    pub transfer_id: Uuid,
    pub from_store_id: Uuid,
    pub to_store_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub quantity: Decimal,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
}

impl CsvRow for TransferRow {
    fn header() -> &'static [&'static str] {
        return &["transfer_id", "from_store_id", "to_store_id", "product_id", "product_name", "quantity", "status", "requested_at", "dispatched_at", "received_at"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.transfer_id.to_string(), self.from_store_id.to_string(), self.to_store_id.to_string(), self.product_id.to_string(),
            self.product_name.clone(), self.quantity.normalize().to_string(), self.status.clone(), self.requested_at.to_rfc3339(),
            self.dispatched_at.map(|t| t.to_rfc3339()).unwrap_or_default(), self.received_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        ];
    }
}

/// `GET /api/v1/reports/transfers?from=&to=&store_id=&product_id=&format=`. Transfers in or out of the stores, by when they were requested.
pub async fn transfers(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, TransferRow>(
        "SELECT t.id AS transfer_id, t.from_store_id, t.to_store_id, t.product_id, p.name AS product_name, t.quantity,
                t.status::text AS status, t.requested_at, t.dispatched_at, t.received_at
         FROM atlas.stock_transfers t JOIN atlas.products p ON p.id = t.product_id
         WHERE (t.from_store_id = ANY($1) OR t.to_store_id = ANY($1))
           AND ($2::timestamptz IS NULL OR t.requested_at >= $2) AND ($3::timestamptz IS NULL OR t.requested_at < $3)
           AND ($4::uuid IS NULL OR t.product_id = $4)
         ORDER BY t.requested_at DESC, t.id DESC",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.product_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "transfers", "Transfers generated.", rows));
}

// === Returns

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReturnRow {
    pub return_id: Uuid,
    pub return_number: Option<String>,
    pub store_id: Uuid,
    pub sale_id: Uuid,
    pub status: String,
    pub reason_code: String,
    pub refund_method: String,
    pub refund_total_minor: i64,
    pub cashier_user_id: Uuid,
    pub requested_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl CsvRow for ReturnRow {
    fn header() -> &'static [&'static str] {
        return &[
            "return_id", "return_number", "store_id", "sale_id", "status", "reason_code", "refund_method", "refund_total_minor",
            "cashier_user_id", "requested_by_user_id", "created_at", "completed_at",
        ];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.return_id.to_string(), opt(&self.return_number), self.store_id.to_string(), self.sale_id.to_string(),
            self.status.clone(), self.reason_code.clone(), self.refund_method.clone(), self.refund_total_minor.to_string(),
            self.cashier_user_id.to_string(), self.requested_by_user_id.to_string(), self.created_at.to_rfc3339(),
            self.completed_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        ];
    }
}

/// `GET /api/v1/reports/returns?from=&to=&store_id=&product_id=&cashier_user_id=&format=`. By when they were raised; `cashier_user_id` is who rang up the sale.
pub async fn returns(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, ReturnRow>(
        "SELECT r.id AS return_id, r.return_number, r.store_id, r.sale_id, r.status::text AS status, r.reason_code,
                r.refund_method::text AS refund_method, r.refund_total_minor, s.cashier_user_id, r.requested_by_user_id,
                r.created_at, r.completed_at
         FROM atlas.returns r JOIN atlas.sales s ON s.id = r.sale_id
         WHERE r.store_id = ANY($1)
           AND ($2::timestamptz IS NULL OR r.created_at >= $2) AND ($3::timestamptz IS NULL OR r.created_at < $3)
           AND ($4::uuid IS NULL OR s.cashier_user_id = $4)
           AND ($5::uuid IS NULL OR EXISTS (
                 SELECT 1 FROM atlas.return_lines rl JOIN atlas.sale_lines sl ON sl.id = rl.sale_line_id
                 WHERE rl.return_id = r.id AND sl.product_id = $5))
         ORDER BY r.created_at DESC, r.id DESC",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.cashier_user_id)
    .bind(query.product_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "returns", "Returns generated.", rows));
}

// === Payments by method

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MethodPayments {
    pub store_id: Uuid,
    pub method: String,
    pub payments_count: i64,
    pub collected_minor: i64,
    pub refunded_minor: i64,
    pub net_minor: i64,
}

impl CsvRow for MethodPayments {
    fn header() -> &'static [&'static str] {
        return &["store_id", "method", "payments_count", "collected_minor", "refunded_minor", "net_minor"];
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.store_id.to_string(), self.method.clone(), self.payments_count.to_string(), self.collected_minor.to_string(),
            self.refunded_minor.to_string(), self.net_minor.to_string(),
        ];
    }
}

/**
 * `GET /api/v1/reports/payments-by-method?from=&to=&store_id=&cashier_user_id=&format=`. Collected is succeeded payments
 * by when they were taken, refunded is executed refunds by when they ran - the same definitions as the summary.
 * `cashier_user_id` is who took the payment.
 */
pub async fn payments_by_method(mut tx: TenantTx, Query(query): Query<ReportQuery>) -> Result<Response, ApiError> {
    let stores = report_stores(&mut tx, query.store_id).await?;
    let rows = sqlx::query_as::<_, MethodPayments>(
        "WITH collected AS (
             SELECT s.store_id, p.method::text AS method, COUNT(*)::bigint AS payments, SUM(p.amount_minor)::bigint AS amount
             FROM atlas.payments p JOIN atlas.sales s ON s.id = p.sale_id
             WHERE s.store_id = ANY($1) AND p.status IN ('succeeded', 'partially_refunded', 'refunded')
               AND ($2::timestamptz IS NULL OR p.created_at >= $2) AND ($3::timestamptz IS NULL OR p.created_at < $3)
               AND ($4::uuid IS NULL OR p.received_by_user_id = $4)
             GROUP BY s.store_id, p.method
         ), refunded AS (
             SELECT s.store_id, pm.kind::text AS method, SUM(r.amount_minor)::bigint AS amount
             FROM atlas.refunds r JOIN atlas.sales s ON s.id = r.sale_id JOIN atlas.payment_methods pm ON pm.id = r.payment_method_id
             WHERE s.store_id = ANY($1) AND r.status = 'executed'
               AND ($2::timestamptz IS NULL OR r.created_at >= $2) AND ($3::timestamptz IS NULL OR r.created_at < $3)
               AND ($4::uuid IS NULL OR r.executed_by_user_id = $4)
             GROUP BY s.store_id, pm.kind
         )
         SELECT COALESCE(c.store_id, f.store_id) AS store_id, COALESCE(c.method, f.method) AS method,
                COALESCE(c.payments, 0)::bigint AS payments_count, COALESCE(c.amount, 0)::bigint AS collected_minor,
                COALESCE(f.amount, 0)::bigint AS refunded_minor, (COALESCE(c.amount, 0) - COALESCE(f.amount, 0))::bigint AS net_minor
         FROM collected c FULL JOIN refunded f ON f.store_id = c.store_id AND f.method = c.method
         ORDER BY store_id, collected_minor DESC",
    )
    .bind(&stores)
    .bind(query.from)
    .bind(query.to)
    .bind(query.cashier_user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(respond(&query, "payments-by-method", "Payments by method generated.", rows));
}
