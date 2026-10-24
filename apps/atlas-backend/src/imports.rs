/*!
 * FR-28 CSV import and export. An import checks every row (required fields, types, identifiers, duplicates in the
 * file and in the business, store association) before writing any of them: one bad row rejects the whole file with
 * an error report, so nothing is ever partly imported. `dry_run=true` checks without writing.
 */
use std::collections::{BTreeMap, BTreeSet};

use axum::extract::{Path, Query};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::audit;
use crate::csv;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::reports::business::{to_csv, CsvRow};
use crate::response::ApiEnvelope;
use crate::roles;
use crate::state::AppState;

const MAX_ROWS: usize = 5_000;

const PRODUCT_COLUMNS: [&str; 6] = ["name", "sku", "product_type", "base_unit", "category", "barcode"];
const SUPPLIER_COLUMNS: [&str; 6] = ["name", "code", "phone", "email", "address", "payment_terms_days"];
const STOCK_COLUMNS: [&str; 4] = ["sku", "quantity", "batch_number", "expiry_date"];

pub fn imports_router() -> Router<AppState> {
    return Router::new()
        .route("/templates/{kind}", get(template))
        .route("/products", post(import_products))
        .route("/suppliers", post(import_suppliers))
        .route("/opening-stock", post(import_opening_stock));
}

pub fn exports_router() -> Router<AppState> {
    return Router::new()
        .route("/products", get(export_products))
        .route("/suppliers", get(export_suppliers));
}

// === Shared

#[derive(Debug, Deserialize)]
pub struct ImportQuery {
    #[serde(default)]
    pub dry_run: bool,
    pub store_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RowError {
    /// The line in the file, counting the header as line 1.
    pub row: usize,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub total_rows: usize,
    pub imported: usize,
    pub dry_run: bool,
}

/// Rows keyed by header name, trimmed. Unknown columns are refused so a typo can't silently drop data.
struct Sheet {
    rows: Vec<BTreeMap<String, String>>,
}

impl Sheet {
    fn parse(text: &str, columns: &[&str], required: &[&str]) -> Result<Sheet, Response> {
        let records = csv::parse(text).map_err(|err| invalid(0, vec![RowError { row: 0, field: String::new(), message: err.0 }]))?;
        let Some((header, body)) = records.split_first() else {
            return Err(invalid(0, vec![RowError { row: 1, field: String::new(), message: "the file is empty".to_string() }]));
        };
        let header: Vec<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
        let mut errors = Vec::new();
        for name in &header {
            if !columns.contains(&name.as_str()) {
                errors.push(RowError { row: 1, field: name.clone(), message: format!("unknown column; expected {}", columns.join(", ")) });
            }
        }
        for name in required {
            if !header.iter().any(|h| h == name) {
                errors.push(RowError { row: 1, field: name.to_string(), message: "required column missing".to_string() });
            }
        }
        if body.len() > MAX_ROWS {
            errors.push(RowError { row: 0, field: String::new(), message: format!("at most {} rows per file", MAX_ROWS) });
        }
        if !errors.is_empty() {
            return Err(invalid(body.len(), errors));
        }
        let rows = body
            .iter()
            .map(|record| {
                header
                    .iter()
                    .enumerate()
                    .map(|(i, name)| (name.clone(), record.get(i).map(|v| v.trim().to_string()).unwrap_or_default()))
                    .collect()
            })
            .collect();
        return Ok(Sheet { rows });
    }
}

fn cell<'a>(row: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    return row.get(name).map(String::as_str).filter(|v| !v.is_empty());
}

fn invalid(total_rows: usize, errors: Vec<RowError>) -> Response {
    return (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "success": false,
            "message": format!("{} problem(s) found; nothing was imported.", errors.len()),
            "code": "IMPORT_INVALID",
            "data": { "total_rows": total_rows, "errors": errors },
        })),
    )
        .into_response();
}

fn imported(message: &str, result: ImportResult) -> Response {
    return Json(ApiEnvelope::ok(message, result)).into_response();
}

fn csv_download(name: &str, body: String) -> Response {
    return (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.csv\"", name)),
        ],
        body,
    )
        .into_response();
}

/// `GET /api/v1/imports/templates/{products|suppliers|opening-stock}`. The header row and one example line.
pub async fn template(Path(kind): Path<String>) -> Result<Response, ApiError> {
    let body = match kind.as_str() {
        "products" => "name,sku,product_type,base_unit,category,barcode\nRice 50kg,RICE-50,Grocery,bag,Staples,6001234567890\n",
        "suppliers" => "name,code,phone,email,address,payment_terms_days\nAcme Foods,ACM-01,+2348011111111,sales@acme.example,12 Wharf Rd Apapa,30\n",
        "opening-stock" => "sku,quantity,batch_number,expiry_date\nRICE-50,40,,\nPARA-500,100,LOT-7,2027-06-30\n",
        _ => return Err(ApiError::not_found("No template by that name; use products, suppliers or opening-stock.")),
    };
    return Ok(csv_download(&format!("{}-template", kind), body.to_string()));
}

// === Products

/// `POST /api/v1/imports/products?dry_run=`. Body is the CSV. Requires `product.manage`.
pub async fn import_products(mut tx: TenantTx, Query(query): Query<ImportQuery>, body: String) -> Result<Response, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "product.manage").await?;
    let sheet = match Sheet::parse(&body, &PRODUCT_COLUMNS, &["name", "sku", "product_type", "base_unit"]) {
        Ok(sheet) => sheet,
        Err(response) => return Ok(response),
    };

    let lookup = |rows: Vec<(String, Uuid)>| -> BTreeMap<String, Uuid> { rows.into_iter().map(|(k, v)| (k.to_lowercase(), v)).collect() };
    let types = lookup(sqlx::query_as("SELECT name, id FROM atlas.product_types WHERE tenant_id = $1").bind(tenant_id).fetch_all(&mut *tx).await?);
    let mut units = lookup(sqlx::query_as("SELECT name, id FROM atlas.units_of_measure WHERE tenant_id = $1").bind(tenant_id).fetch_all(&mut *tx).await?);
    units.extend(lookup(sqlx::query_as("SELECT abbreviation, id FROM atlas.units_of_measure WHERE tenant_id = $1").bind(tenant_id).fetch_all(&mut *tx).await?));
    let categories = lookup(
        sqlx::query_as("SELECT name, id FROM atlas.product_categories WHERE tenant_id = $1 AND archived_at IS NULL").bind(tenant_id).fetch_all(&mut *tx).await?,
    );
    let existing_skus: BTreeSet<String> = sqlx::query_scalar::<_, String>("SELECT lower(sku) FROM atlas.products WHERE tenant_id = $1 AND sku IS NOT NULL")
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .collect();
    let existing_barcodes: BTreeSet<String> = sqlx::query_scalar::<_, String>("SELECT barcode FROM atlas.product_barcodes WHERE tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .collect();

    struct NewProduct {
        name: String,
        sku: String,
        product_type_id: Uuid,
        base_unit_id: Uuid,
        category_id: Option<Uuid>,
        barcode: Option<String>,
    }
    let mut errors = Vec::new();
    let mut seen_skus = BTreeSet::new();
    let mut seen_barcodes = BTreeSet::new();
    let mut valid = Vec::new();
    for (index, row) in sheet.rows.iter().enumerate() {
        let line = index + 2;
        let mut fail = |field: &str, message: String| errors.push(RowError { row: line, field: field.to_string(), message });
        let name = cell(row, "name");
        let sku = cell(row, "sku");
        if name.is_none() {
            fail("name", "required".to_string());
        }
        match sku {
            None => fail("sku", "required".to_string()),
            Some(sku) if existing_skus.contains(&sku.to_lowercase()) => fail("sku", format!("{} already exists", sku)),
            Some(sku) if !seen_skus.insert(sku.to_lowercase()) => fail("sku", format!("{} appears more than once in this file", sku)),
            Some(_) => {}
        }
        let product_type_id = match cell(row, "product_type") {
            None => {
                fail("product_type", "required".to_string());
                None
            }
            Some(t) => types.get(&t.to_lowercase()).copied().or_else(|| {
                fail("product_type", format!("no product type named {}", t));
                None
            }),
        };
        let base_unit_id = match cell(row, "base_unit") {
            None => {
                fail("base_unit", "required".to_string());
                None
            }
            Some(u) => units.get(&u.to_lowercase()).copied().or_else(|| {
                fail("base_unit", format!("no unit named {}", u));
                None
            }),
        };
        let category_id = match cell(row, "category") {
            None => None,
            Some(c) => categories.get(&c.to_lowercase()).copied().or_else(|| {
                fail("category", format!("no active category named {}", c));
                None
            }),
        };
        let barcode = cell(row, "barcode").map(str::to_string);
        if let Some(code) = &barcode {
            if existing_barcodes.contains(code) {
                fail("barcode", format!("{} is already used", code));
            } else if !seen_barcodes.insert(code.clone()) {
                fail("barcode", format!("{} appears more than once in this file", code));
            }
        }
        if let (Some(name), Some(sku), Some(product_type_id), Some(base_unit_id)) = (name, sku, product_type_id, base_unit_id) {
            valid.push(NewProduct { name: name.to_string(), sku: sku.to_string(), product_type_id, base_unit_id, category_id, barcode });
        }
    }
    if !errors.is_empty() {
        return Ok(invalid(sheet.rows.len(), errors));
    }

    if !query.dry_run {
        for product in &valid {
            let product_id = Uuid::now_v7();
            sqlx::query(
                "INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name, sku, category_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(product_id)
            .bind(tenant_id)
            .bind(product.product_type_id)
            .bind(product.base_unit_id)
            .bind(&product.name)
            .bind(&product.sku)
            .bind(product.category_id)
            .execute(&mut *tx)
            .await?;
            if let Some(code) = &product.barcode {
                sqlx::query("INSERT INTO atlas.product_barcodes (id, tenant_id, product_id, unit_id, barcode) VALUES ($1, $2, $3, $4, $5)")
                    .bind(Uuid::now_v7())
                    .bind(tenant_id)
                    .bind(product_id)
                    .bind(product.base_unit_id)
                    .bind(code)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        audit::record(&mut tx, tenant_id, user_id, "import", Uuid::now_v7(), "import.products", Some(json!({ "rows": valid.len() }))).await?;
    }
    tx.commit().await?;
    let result = ImportResult { total_rows: sheet.rows.len(), imported: if query.dry_run { 0 } else { valid.len() }, dry_run: query.dry_run };
    return Ok(imported(if query.dry_run { "File is valid; nothing was written." } else { "Products imported." }, result));
}

// === Suppliers

/// `POST /api/v1/imports/suppliers?dry_run=`. Requires `supplier.manage`.
pub async fn import_suppliers(mut tx: TenantTx, Query(query): Query<ImportQuery>, body: String) -> Result<Response, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "supplier.manage").await?;
    let sheet = match Sheet::parse(&body, &SUPPLIER_COLUMNS, &["name"]) {
        Ok(sheet) => sheet,
        Err(response) => return Ok(response),
    };
    let existing_codes: BTreeSet<String> =
        sqlx::query_scalar::<_, String>("SELECT lower(code) FROM atlas.suppliers WHERE tenant_id = $1 AND code IS NOT NULL")
            .bind(tenant_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut valid: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>)> = Vec::new();
    for (index, row) in sheet.rows.iter().enumerate() {
        let line = index + 2;
        let mut fail = |field: &str, message: String| errors.push(RowError { row: line, field: field.to_string(), message });
        let name = cell(row, "name");
        if name.is_none() {
            fail("name", "required".to_string());
        }
        let code = cell(row, "code").map(str::to_string);
        if let Some(code) = &code {
            if existing_codes.contains(&code.to_lowercase()) {
                fail("code", format!("{} already exists", code));
            } else if !seen.insert(code.to_lowercase()) {
                fail("code", format!("{} appears more than once in this file", code));
            }
        }
        let email = cell(row, "email").map(str::to_string);
        if email.as_deref().is_some_and(|e| !e.contains('@')) {
            fail("email", "not an email address".to_string());
        }
        let terms = match cell(row, "payment_terms_days") {
            None => None,
            Some(value) => match value.parse::<i32>() {
                Ok(days) if days >= 0 => Some(days),
                _ => {
                    fail("payment_terms_days", "must be a whole number of days, 0 or more".to_string());
                    None
                }
            },
        };
        if let Some(name) = name {
            valid.push((name.to_string(), code, cell(row, "phone").map(str::to_string), email, cell(row, "address").map(str::to_string), terms));
        }
    }
    if !errors.is_empty() {
        return Ok(invalid(sheet.rows.len(), errors));
    }

    if !query.dry_run {
        for (name, code, phone, email, address, terms) in &valid {
            sqlx::query(
                "INSERT INTO atlas.suppliers (id, tenant_id, name, code, phone, email, address, payment_terms_days)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(Uuid::now_v7())
            .bind(tenant_id)
            .bind(name)
            .bind(code)
            .bind(phone)
            .bind(email)
            .bind(address)
            .bind(terms)
            .execute(&mut *tx)
            .await?;
        }
        audit::record(&mut tx, tenant_id, user_id, "import", Uuid::now_v7(), "import.suppliers", Some(json!({ "rows": valid.len() }))).await?;
    }
    tx.commit().await?;
    let result = ImportResult { total_rows: sheet.rows.len(), imported: if query.dry_run { 0 } else { valid.len() }, dry_run: query.dry_run };
    return Ok(imported(if query.dry_run { "File is valid; nothing was written." } else { "Suppliers imported." }, result));
}

// === Opening stock

/// `POST /api/v1/imports/opening-stock?store_id=&dry_run=`. Requires `stock.adjust` at the store; each row becomes an adjustment.
pub async fn import_opening_stock(mut tx: TenantTx, Query(query): Query<ImportQuery>, body: String) -> Result<Response, ApiError> {
    let store_id = query.store_id.ok_or_else(|| ApiError::bad_request_with_code("store_id is required", "VALIDATION_FAILED"))?;
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "stock.adjust").await?;
    crate::validate::require_visible(&mut tx, "stores", store_id, "Store").await?;
    let sheet = match Sheet::parse(&body, &STOCK_COLUMNS, &["sku", "quantity"]) {
        Ok(sheet) => sheet,
        Err(response) => return Ok(response),
    };
    let products: BTreeMap<String, (Uuid, bool)> = sqlx::query_as::<_, (String, Uuid, bool)>(
        "SELECT lower(p.sku), p.id, pt.identity_mode IN ('by_batch', 'by_both')
         FROM atlas.products p JOIN atlas.product_types pt ON pt.id = p.product_type_id
         WHERE p.tenant_id = $1 AND p.sku IS NOT NULL AND p.deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|(sku, id, batched)| (sku, (id, batched)))
    .collect();

    struct StockRow {
        product_id: Uuid,
        quantity: Decimal,
        batch: Option<(String, Option<NaiveDate>)>,
    }
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut valid = Vec::new();
    for (index, row) in sheet.rows.iter().enumerate() {
        let line = index + 2;
        let mut fail = |field: &str, message: String| errors.push(RowError { row: line, field: field.to_string(), message });
        let product = match cell(row, "sku") {
            None => {
                fail("sku", "required".to_string());
                None
            }
            Some(sku) => products.get(&sku.to_lowercase()).copied().or_else(|| {
                fail("sku", format!("no product with SKU {}", sku));
                None
            }),
        };
        let quantity = match cell(row, "quantity").map(str::parse::<Decimal>) {
            Some(Ok(q)) if q > Decimal::ZERO => Some(q),
            _ => {
                fail("quantity", "must be a number above zero".to_string());
                None
            }
        };
        let expiry = match cell(row, "expiry_date") {
            None => None,
            Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d").ok().or_else(|| {
                fail("expiry_date", "must be YYYY-MM-DD".to_string());
                None
            }),
        };
        let batch_number = cell(row, "batch_number").map(str::to_string);
        if let Some((product_id, batched)) = product {
            match (&batch_number, batched) {
                (None, true) => fail("batch_number", "this product is batch-tracked; name the batch".to_string()),
                (Some(_), false) => fail("batch_number", "this product isn't batch-tracked; leave it empty".to_string()),
                _ => {}
            }
            let key = format!("{}:{}", product_id, batch_number.clone().unwrap_or_default().to_lowercase());
            if !seen.insert(key) {
                fail("sku", "the same product and batch appear more than once".to_string());
            }
            if let Some(quantity) = quantity {
                valid.push(StockRow { product_id, quantity, batch: batch_number.map(|b| (b, expiry)) });
            }
        }
    }
    if !errors.is_empty() {
        return Ok(invalid(sheet.rows.len(), errors));
    }

    if !query.dry_run {
        let reference = Uuid::now_v7();
        for row in &valid {
            let stock_item_id = match &row.batch {
                Some((number, expiry)) => Some(inventory::batches::find_or_create_batch(&mut tx, tenant_id, row.product_id, number, *expiry).await?),
                None => None,
            };
            let lines = [PostingLine { product_id: row.product_id, stock_item_id, unit_id: None, quantity: row.quantity }];
            inventory::service::post(
                &mut tx,
                Posting {
                    tenant_id,
                    store_id,
                    movement_type: MovementType::Adjustment,
                    reason: "Opening stock import",
                    reference_id: Some(reference),
                    actor_user_id: user_id,
                    policy: NegativeStockPolicy::Allow,
                    source: "import",
                    lines: &lines,
                },
            )
            .await?;
        }
        audit::record(&mut tx, tenant_id, user_id, "import", reference, "import.opening_stock", Some(json!({ "store_id": store_id, "rows": valid.len() }))).await?;
    }
    tx.commit().await?;
    let result = ImportResult { total_rows: sheet.rows.len(), imported: if query.dry_run { 0 } else { valid.len() }, dry_run: query.dry_run };
    return Ok(imported(if query.dry_run { "File is valid; nothing was written." } else { "Opening stock imported." }, result));
}

// === Exports

#[derive(Debug, sqlx::FromRow)]
struct ProductExport {
    name: String,
    sku: Option<String>,
    product_type: String,
    base_unit: String,
    category: Option<String>,
    barcode: Option<String>,
}

impl CsvRow for ProductExport {
    fn header() -> &'static [&'static str] {
        return &PRODUCT_COLUMNS;
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.name.clone(),
            self.sku.clone().unwrap_or_default(),
            self.product_type.clone(),
            self.base_unit.clone(),
            self.category.clone().unwrap_or_default(),
            self.barcode.clone().unwrap_or_default(),
        ];
    }
}

/// `GET /api/v1/exports/products`. In the import template's own columns, so it can be edited and re-imported elsewhere.
pub async fn export_products(mut tx: TenantTx) -> Result<Response, ApiError> {
    let tenant_id = tx.tenant_id;
    let rows = sqlx::query_as::<_, ProductExport>(
        "SELECT p.name, p.sku, pt.name AS product_type, u.abbreviation AS base_unit, c.name AS category,
                (SELECT b.barcode FROM atlas.product_barcodes b WHERE b.product_id = p.id ORDER BY b.created_at LIMIT 1) AS barcode
         FROM atlas.products p
         JOIN atlas.product_types pt ON pt.id = p.product_type_id
         JOIN atlas.units_of_measure u ON u.id = p.base_unit_id
         LEFT JOIN atlas.product_categories c ON c.id = p.category_id
         WHERE p.tenant_id = $1 AND p.deleted_at IS NULL
         ORDER BY p.name",
    )
    .bind(tenant_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(csv_download("products", to_csv(&rows)));
}

#[derive(Debug, sqlx::FromRow)]
struct SupplierExport {
    name: String,
    code: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    payment_terms_days: Option<i32>,
}

impl CsvRow for SupplierExport {
    fn header() -> &'static [&'static str] {
        return &SUPPLIER_COLUMNS;
    }

    fn cells(&self) -> Vec<String> {
        return vec![
            self.name.clone(),
            self.code.clone().unwrap_or_default(),
            self.phone.clone().unwrap_or_default(),
            self.email.clone().unwrap_or_default(),
            self.address.clone().unwrap_or_default(),
            self.payment_terms_days.map(|d| d.to_string()).unwrap_or_default(),
        ];
    }
}

/// `GET /api/v1/exports/suppliers`. Requires `supplier.manage`, since it includes contact details.
pub async fn export_suppliers(mut tx: TenantTx) -> Result<Response, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "supplier.manage").await?;
    let rows = sqlx::query_as::<_, SupplierExport>(
        "SELECT name, code, phone, email, address, payment_terms_days FROM atlas.suppliers
         WHERE tenant_id = $1 AND deleted_at IS NULL ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(csv_download("suppliers", to_csv(&rows)));
}
