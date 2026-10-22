use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::inventory;
use crate::pricing::models::StorePricing;
use crate::pricing::resolver;

use super::models::{CatalogItem, CatalogUnit};

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    name: String,
    sku: Option<String>,
    identity_mode: String,
    is_returnable: bool,
    base_unit_id: Uuid,
    tax_rate_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct UnitRow {
    id: Uuid,
    name: String,
    abbreviation: String,
    factor_to_base: Decimal,
}

const LISTED_PRODUCTS: &str = "SELECT p.id, p.name, p.sku, pt.identity_mode::text AS identity_mode, p.is_returnable,
            p.base_unit_id, p.tax_rate_id
     FROM atlas.products p
     JOIN atlas.product_types pt ON pt.id = p.product_type_id
     JOIN atlas.product_store_listings psl ON psl.product_id = p.id AND psl.store_id = $1 AND psl.is_active
     WHERE p.deleted_at IS NULL AND p.is_active";

/**
 * Name or SKU substring match, plus an exact barcode match that always sorts first - a scanner typed into the search box should land on its product, not on everything whose name happens to contain the digits.
 */
pub async fn search(
    conn: &mut PgConnection,
    store: &StorePricing,
    q: Option<&str>,
    limit: i64,
) -> Result<Vec<CatalogItem>, ApiError> {
    let term = q.map(str::trim).filter(|t| !t.is_empty());
    let pattern = term.map(|t| format!("%{}%", t));
    let rows = sqlx::query_as::<_, ProductRow>(&format!(
        "{}
           AND ($2::text IS NULL OR p.name ILIKE $3 OR p.sku ILIKE $3
                OR EXISTS (SELECT 1 FROM atlas.product_barcodes b WHERE b.product_id = p.id AND b.barcode = $2))
         ORDER BY EXISTS (SELECT 1 FROM atlas.product_barcodes b WHERE b.product_id = p.id AND b.barcode = $2) DESC, p.name
         LIMIT $4",
        LISTED_PRODUCTS
    ))
    .bind(store.id)
    .bind(term)
    .bind(pattern)
    .bind(limit)
    .fetch_all(&mut *conn)
    .await?;

    return build_items(conn, store, rows).await;
}

/// `(unit_id, item)` for an exact barcode, or `None` if the barcode is unknown or its product isn't sold at this store.
pub async fn by_barcode(
    conn: &mut PgConnection,
    store: &StorePricing,
    code: &str,
) -> Result<Option<(Uuid, CatalogItem)>, ApiError> {
    let hit: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT product_id, unit_id FROM atlas.product_barcodes WHERE barcode = $1")
            .bind(code.trim())
            .fetch_optional(&mut *conn)
            .await?;
    let Some((product_id, unit_id)) = hit else {
        return Ok(None);
    };
    let rows = sqlx::query_as::<_, ProductRow>(&format!("{} AND p.id = $2", LISTED_PRODUCTS))
        .bind(store.id)
        .bind(product_id)
        .fetch_all(&mut *conn)
        .await?;
    let mut items = build_items(conn, store, rows).await?;
    return Ok(items.pop().map(|item| (unit_id, item)));
}

/// Every listed product, or only those whose product row, price rules or barcodes changed after `updated_since`.
pub async fn snapshot_items(
    conn: &mut PgConnection,
    store: &StorePricing,
    updated_since: Option<DateTime<Utc>>,
) -> Result<Vec<CatalogItem>, ApiError> {
    let rows = sqlx::query_as::<_, ProductRow>(&format!(
        "{}
           AND ($2::timestamptz IS NULL
                OR p.updated_at > $2
                OR EXISTS (SELECT 1 FROM atlas.price_rules pr WHERE pr.product_id = p.id AND pr.updated_at > $2)
                OR EXISTS (SELECT 1 FROM atlas.product_barcodes b WHERE b.product_id = p.id AND b.updated_at > $2))
         ORDER BY p.name",
        LISTED_PRODUCTS
    ))
    .bind(store.id)
    .bind(updated_since)
    .fetch_all(&mut *conn)
    .await?;
    return build_items(conn, store, rows).await;
}

async fn build_items(conn: &mut PgConnection, store: &StorePricing, rows: Vec<ProductRow>) -> Result<Vec<CatalogItem>, ApiError> {
    let now = Utc::now();
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let units = sqlx::query_as::<_, UnitRow>(
            "SELECT u.id, u.name, u.abbreviation, 1::numeric AS factor_to_base
             FROM atlas.units_of_measure u WHERE u.id = $2
             UNION ALL
             SELECT u.id, u.name, u.abbreviation, psu.factor_to_base
             FROM atlas.product_sale_units psu
             JOIN atlas.units_of_measure u ON u.id = psu.unit_id
             WHERE psu.product_id = $1 AND psu.unit_id <> $2",
        )
        .bind(row.id)
        .bind(row.base_unit_id)
        .fetch_all(&mut *conn)
        .await?;
        let barcodes: Vec<(Uuid, String)> =
            sqlx::query_as("SELECT unit_id, barcode FROM atlas.product_barcodes WHERE product_id = $1 ORDER BY barcode")
                .bind(row.id)
                .fetch_all(&mut *conn)
                .await?;

        let mut catalog_units = Vec::with_capacity(units.len());
        for unit in units {
            let price = resolver::resolve_base_price(conn, row.id, store.id, None, unit.factor_to_base, now)
                .await?
                .map(|r| crate::pricing::engine::round_minor(Decimal::from(r.base_unit_price_minor) * unit.factor_to_base));
            catalog_units.push(CatalogUnit {
                unit_id: unit.id,
                barcodes: barcodes.iter().filter(|(u, _)| *u == unit.id).map(|(_, b)| b.clone()).collect(),
                name: unit.name,
                abbreviation: unit.abbreviation,
                factor_to_base: unit.factor_to_base,
                unit_price_amount_minor: price,
            });
        }

        let (tax_rate_id, tax_rate_basis_points) = resolver::effective_tax_rate(conn, row.tax_rate_id, store).await?;
        let on_hand = inventory::service::on_hand(conn, store.id, row.id, None).await?;
        items.push(CatalogItem {
            product_id: row.id,
            name: row.name,
            sku: row.sku,
            identity_mode: row.identity_mode,
            is_returnable: row.is_returnable,
            base_unit_id: row.base_unit_id,
            tax_rate_id,
            tax_rate_basis_points,
            on_hand,
            units: catalog_units,
        });
    }
    return Ok(items);
}
