/*!
 * The public catalogue: products listed and active at the storefront's store, priced by the same resolver as the
 * till, with availability net of stock held for unpaid orders. Also recommendations and recently viewed.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::ApiError;
use crate::pricing::resolver;
use crate::response::ApiEnvelope;

use super::context::ShopTx;
use super::promotions;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 100;
const RECOMMENDATION_LIMIT: i64 = 8;

// === Shared lookups

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SellableProduct {
    pub id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub attributes: Value,
    pub category_id: Option<Uuid>,
    pub tax_rate_id: Option<Uuid>,
    pub unit_cost_minor: Option<i64>,
}

/// A product the store sells online: active, listed and active at the store.
pub async fn sellable(conn: &mut PgConnection, store_id: Uuid, product_id: Uuid) -> Result<Option<SellableProduct>, ApiError> {
    return sqlx::query_as::<_, SellableProduct>(
        "SELECT p.id, p.name, p.sku, p.attributes, p.category_id, p.tax_rate_id, l.cost_amount_minor AS unit_cost_minor
         FROM atlas.products p
         JOIN atlas.product_store_listings l ON l.product_id = p.id AND l.store_id = $2 AND l.is_active
         WHERE p.id = $1 AND p.is_active AND p.deleted_at IS NULL",
    )
    .bind(product_id)
    .bind(store_id)
    .fetch_optional(conn)
    .await
    .map_err(ApiError::from);
}

/// On hand less what active, unexpired reservations hold.
pub async fn available(conn: &mut PgConnection, store_id: Uuid, product_id: Uuid) -> Result<Decimal, ApiError> {
    return sqlx::query_scalar(
        "SELECT COALESCE((SELECT SUM(quantity) FROM atlas.stock_movements WHERE store_id = $1 AND product_id = $2), 0)
              - COALESCE((SELECT SUM(quantity) FROM atlas.stock_reservations
                          WHERE store_id = $1 AND product_id = $2 AND status = 'active'
                            AND (expires_at IS NULL OR expires_at > now())), 0)",
    )
    .bind(store_id)
    .bind(product_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn customer_group(conn: &mut PgConnection, shopper_id: Option<Uuid>) -> Result<Option<Uuid>, ApiError> {
    let Some(shopper_id) = shopper_id else { return Ok(None) };
    let group: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT c.customer_group_id FROM atlas.shoppers s JOIN atlas.customers c ON c.id = s.customer_id WHERE s.id = $1",
    )
    .bind(shopper_id)
    .fetch_optional(conn)
    .await?;
    return Ok(group.flatten());
}

/// The regular price per base unit for one unit, or `None` when the product has no price at this store.
pub async fn regular_price(conn: &mut PgConnection, store_id: Uuid, product_id: Uuid, group: Option<Uuid>, quantity: Decimal) -> Result<Option<i64>, ApiError> {
    let resolved = resolver::resolve_base_price(conn, product_id, store_id, group, quantity, Utc::now()).await?;
    return Ok(resolved.map(|r| r.base_unit_price_minor));
}

pub fn weight_kg(attributes: &Value) -> Decimal {
    return match attributes.get("weight_kg") {
        Some(Value::Number(n)) => n.to_string().parse().unwrap_or(Decimal::ZERO),
        Some(Value::String(s)) => s.parse().unwrap_or(Decimal::ZERO),
        _ => Decimal::ZERO,
    };
}

/// A shopper's id, or the hash of a guest's cart token: who "recently viewed" is about.
pub fn viewer_key(tx: &ShopTx) -> Option<String> {
    if let Some(shopper_id) = tx.shopper_id {
        return Some(shopper_id.to_string());
    }
    return tx.cart_token.as_deref().map(crypto::hash_lookup_token);
}

// === Listing

#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    pub q: Option<String>,
    pub category_id: Option<Uuid>,
    pub min_price_minor: Option<i64>,
    pub max_price_minor: Option<i64>,
    #[serde(default)]
    pub in_stock: bool,
    /// `name` (default), `price_asc`, `price_desc` or `newest`.
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CatalogProduct {
    pub id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub category_id: Option<Uuid>,
    pub attributes: Value,
    pub price_minor: i64,
    /// Set while a flash sale covers it.
    pub sale_price_minor: Option<i64>,
    pub available: Decimal,
    pub in_stock: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct ListedRow {
    id: Uuid,
    name: String,
    sku: Option<String>,
    category_id: Option<Uuid>,
    attributes: Value,
}

pub async fn describe(tx: &mut ShopTx, row_id: Uuid, name: String, sku: Option<String>, category_id: Option<Uuid>, attributes: Value) -> Result<Option<CatalogProduct>, ApiError> {
    let (store_id, shopper_id) = (tx.store_id, tx.shopper_id);
    let group = customer_group(tx, shopper_id).await?;
    let Some(price) = regular_price(tx, store_id, row_id, group, Decimal::ONE).await? else { return Ok(None) };
    let sale = promotions::flash_price(tx, store_id, row_id).await?.filter(|flash| *flash < price);
    let stock = available(tx, store_id, row_id).await?;
    return Ok(Some(CatalogProduct {
        id: row_id,
        name,
        sku,
        category_id,
        attributes,
        price_minor: price,
        sale_price_minor: sale,
        in_stock: stock > Decimal::ZERO,
        available: stock.max(Decimal::ZERO),
    }));
}

/// `GET /api/v1/shop/:handle/products?q=&category_id=&min_price_minor=&max_price_minor=&in_stock=&sort=&limit=&offset=`.
pub async fn list_products(mut tx: ShopTx, Query(query): Query<CatalogQuery>) -> Result<Json<ApiEnvelope<Vec<CatalogProduct>>>, ApiError> {
    let store_id = tx.store_id;
    let search = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()).map(str::to_string);
    let rows = sqlx::query_as::<_, ListedRow>(
        "SELECT p.id, p.name, p.sku, p.category_id, p.attributes
         FROM atlas.products p
         JOIN atlas.product_store_listings l ON l.product_id = p.id AND l.store_id = $1 AND l.is_active
         WHERE p.is_active AND p.deleted_at IS NULL
           AND ($2::text IS NULL OR p.name ILIKE '%' || $2 || '%' OR p.sku ILIKE '%' || $2 || '%')
           AND ($3::uuid IS NULL OR p.category_id = $3)
         ORDER BY CASE WHEN $4 = 'newest' THEN p.created_at END DESC NULLS LAST, p.name",
    )
    .bind(store_id)
    .bind(&search)
    .bind(query.category_id)
    .bind(query.sort.as_deref().unwrap_or("name"))
    .fetch_all(&mut *tx)
    .await?;

    let mut products = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(product) = describe(&mut tx, row.id, row.name, row.sku, row.category_id, row.attributes).await? else { continue };
        let effective = product.sale_price_minor.unwrap_or(product.price_minor);
        if query.min_price_minor.is_some_and(|min| effective < min)
            || query.max_price_minor.is_some_and(|max| effective > max)
            || (query.in_stock && !product.in_stock)
        {
            continue;
        }
        products.push(product);
    }
    match query.sort.as_deref() {
        Some("price_asc") => products.sort_by_key(|p| p.sale_price_minor.unwrap_or(p.price_minor)),
        Some("price_desc") => products.sort_by_key(|p| std::cmp::Reverse(p.sale_price_minor.unwrap_or(p.price_minor))),
        _ => {}
    }
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
    let page: Vec<CatalogProduct> = products.into_iter().skip(offset).take(limit).collect();
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Products fetched.", page)));
}

/// `GET /api/v1/shop/:handle/products/:product_id`. Also records the view for "recently viewed".
pub async fn show_product(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<CatalogProduct>>, ApiError> {
    let store_id = tx.store_id;
    let product = sellable(&mut tx, store_id, product_id).await?.ok_or_else(|| ApiError::not_found("Product not found."))?;
    let shown = describe(&mut tx, product.id, product.name, product.sku, product.category_id, product.attributes)
        .await?
        .ok_or_else(|| ApiError::not_found("Product not found."))?;
    if let Some(viewer) = viewer_key(&tx) {
        let tenant_id = tx.tenant_id;
        sqlx::query("INSERT INTO atlas.product_views (id, tenant_id, store_id, viewer_key, product_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(Uuid::now_v7())
            .bind(tenant_id)
            .bind(store_id)
            .bind(&viewer)
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product fetched.", shown)));
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShopCategory {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// `GET /api/v1/shop/:handle/categories`. Active categories that have something on sale here.
pub async fn list_categories(mut tx: ShopTx) -> Result<Json<ApiEnvelope<Vec<ShopCategory>>>, ApiError> {
    let store_id = tx.store_id;
    let categories = sqlx::query_as::<_, ShopCategory>(
        "SELECT DISTINCT c.id, c.name, c.description
         FROM atlas.product_categories c
         JOIN atlas.products p ON p.category_id = c.id AND p.is_active AND p.deleted_at IS NULL
         JOIN atlas.product_store_listings l ON l.product_id = p.id AND l.store_id = $1 AND l.is_active
         WHERE c.archived_at IS NULL
         ORDER BY c.name",
    )
    .bind(store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Categories fetched.", categories)));
}

// === Recommendations

#[derive(Debug, Serialize)]
pub struct Recommendations {
    pub related: Vec<CatalogProduct>,
    pub frequently_bought_together: Vec<CatalogProduct>,
    pub recently_viewed: Vec<CatalogProduct>,
}

async fn describe_ids(tx: &mut ShopTx, ids: Vec<Uuid>) -> Result<Vec<CatalogProduct>, ApiError> {
    let store_id = tx.store_id;
    let mut products = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(product) = sellable(tx, store_id, id).await? {
            if let Some(shown) = describe(tx, product.id, product.name, product.sku, product.category_id, product.attributes).await? {
                products.push(shown);
            }
        }
    }
    return Ok(products);
}

/**
 * `GET /api/v1/shop/:handle/products/:product_id/recommendations`.
 * Related is the same category, in stock first; frequently bought together counts how often a product shared an
 * order or till sale with this one; recently viewed is this shopper's or guest's own trail.
 */
pub async fn recommendations(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<Recommendations>>, ApiError> {
    let store_id = tx.store_id;
    let product = sellable(&mut tx, store_id, product_id).await?.ok_or_else(|| ApiError::not_found("Product not found."))?;

    let related_ids: Vec<Uuid> = match product.category_id {
        None => Vec::new(),
        Some(category_id) => sqlx::query_scalar(
            "SELECT p.id FROM atlas.products p
             JOIN atlas.product_store_listings l ON l.product_id = p.id AND l.store_id = $1 AND l.is_active
             WHERE p.category_id = $2 AND p.id <> $3 AND p.is_active AND p.deleted_at IS NULL
             ORDER BY COALESCE((SELECT SUM(m.quantity) FROM atlas.stock_movements m WHERE m.store_id = $1 AND m.product_id = p.id), 0) > 0 DESC, p.name
             LIMIT $4",
        )
        .bind(store_id)
        .bind(category_id)
        .bind(product_id)
        .bind(RECOMMENDATION_LIMIT)
        .fetch_all(&mut *tx)
        .await?,
    };

    let together_ids: Vec<Uuid> = sqlx::query_scalar(
        "WITH baskets AS (
             SELECT l.order_id AS basket, l.product_id FROM atlas.shop_order_lines l
             JOIN atlas.shop_orders o ON o.id = l.order_id
             WHERE o.store_id = $1 AND o.status IN ('paid', 'processing', 'shipped', 'delivered')
             UNION ALL
             SELECT sl.sale_id, sl.product_id FROM atlas.sale_lines sl
             JOIN atlas.sales s ON s.id = sl.sale_id
             WHERE s.store_id = $1 AND s.status = 'completed'
         )
         SELECT other.product_id FROM baskets mine
         JOIN baskets other ON other.basket = mine.basket AND other.product_id <> mine.product_id
         WHERE mine.product_id = $2
         GROUP BY other.product_id
         ORDER BY COUNT(DISTINCT other.basket) DESC, other.product_id
         LIMIT $3",
    )
    .bind(store_id)
    .bind(product_id)
    .bind(RECOMMENDATION_LIMIT)
    .fetch_all(&mut *tx)
    .await?;

    let recent_ids: Vec<Uuid> = match viewer_key(&tx) {
        None => Vec::new(),
        Some(viewer) => sqlx::query_scalar(
            "SELECT product_id FROM atlas.product_views
             WHERE store_id = $1 AND viewer_key = $2 AND product_id <> $3
             GROUP BY product_id ORDER BY MAX(viewed_at) DESC LIMIT $4",
        )
        .bind(store_id)
        .bind(&viewer)
        .bind(product_id)
        .bind(RECOMMENDATION_LIMIT)
        .fetch_all(&mut *tx)
        .await?,
    };

    let recommendations = Recommendations {
        related: describe_ids(&mut tx, related_ids).await?,
        frequently_bought_together: describe_ids(&mut tx, together_ids).await?,
        recently_viewed: describe_ids(&mut tx, recent_ids).await?,
    };
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Recommendations fetched.", recommendations)));
}

// === Shop info

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicZone {
    pub id: Uuid,
    pub name: String,
    pub regions: Vec<String>,
    pub base_fee_minor: i64,
    pub per_kg_fee_minor: i64,
    pub allows_pay_on_delivery: bool,
}

#[derive(Debug, Serialize)]
pub struct ShopInfo {
    pub handle: String,
    pub store_name: String,
    pub currency: String,
    pub prices_include_tax: bool,
    pub shipping_zones: Vec<PublicZone>,
    pub pay_on_delivery_max_minor: Option<i64>,
    pub loyalty_enabled: bool,
    pub loyalty_point_value_minor: i64,
}

/// `GET /api/v1/shop/:handle`. What a storefront needs before showing anything: name, currency, delivery zones.
pub async fn shop_info(mut tx: ShopTx) -> Result<Json<ApiEnvelope<ShopInfo>>, ApiError> {
    let store_id = tx.store_id;
    let (store_name, currency, prices_include_tax): (String, String, bool) =
        sqlx::query_as("SELECT name, currency::text, prices_include_tax FROM atlas.stores WHERE id = $1")
            .bind(store_id)
            .fetch_one(&mut *tx)
            .await?;
    let shipping_zones = sqlx::query_as::<_, PublicZone>(
        "SELECT id, name, regions, base_fee_minor, per_kg_fee_minor, allows_pay_on_delivery FROM atlas.shipping_zones
         WHERE store_id = $1 AND is_active ORDER BY name",
    )
    .bind(store_id)
    .fetch_all(&mut *tx)
    .await?;
    let info = ShopInfo {
        handle: tx.storefront.handle.clone(),
        store_name,
        currency,
        prices_include_tax,
        shipping_zones,
        pay_on_delivery_max_minor: tx.storefront.pay_on_delivery_max_minor,
        loyalty_enabled: tx.storefront.loyalty_minor_per_point.is_some(),
        loyalty_point_value_minor: tx.storefront.loyalty_point_value_minor,
    };
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shop fetched.", info)));
}
