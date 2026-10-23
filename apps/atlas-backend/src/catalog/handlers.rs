use axum::extract::{Path, Query};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::pricing::models::TaxRate;
use crate::pricing::quote::{self, QuoteContext};
use crate::pricing::{resolver, Quote, QuoteRequest};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{BarcodeMatch, CatalogItem, CatalogSnapshot, SearchQuery, SnapshotPriceRule, SnapshotQuery};
use super::service;

async fn assert_can_sell(tx: &mut TenantTx, store_id: Uuid) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), "sale.create").await;
}

/**
 * `GET /api/v1/stores/:id/catalog?q=&limit=`.
 * Requires `sale.create` at the store. Only active products listed at this store come back, each with its units, walk-in price, tax rate and on-hand.
 */
pub async fn search(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiEnvelope<Vec<CatalogItem>>>, ApiError> {
    assert_can_sell(&mut tx, store_id).await?;
    let store = resolver::load_store_pricing(&mut tx, store_id).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let items = service::search(&mut tx, &store, query.q.as_deref(), limit).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Catalog fetched.", items)));
}

/// `GET /api/v1/stores/:id/catalog/barcode/:code`. 404 for an unknown barcode or one whose product isn't sold here.
pub async fn by_barcode(
    Path((store_id, code)): Path<(Uuid, String)>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<BarcodeMatch>>, ApiError> {
    assert_can_sell(&mut tx, store_id).await?;
    let store = resolver::load_store_pricing(&mut tx, store_id).await?;
    let (unit_id, item) = service::by_barcode(&mut tx, &store, &code)
        .await?
        .ok_or_else(|| ApiError::not_found("No product with that barcode is sold at this store."))?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Barcode matched.", BarcodeMatch { unit_id, item })));
}

/**
 * `POST /api/v1/stores/:id/catalog/quote`.
 * Prices a cart without saving anything - what the till calls after every add, remove or quantity change to show the total. Policy problems come back as `violations`, not errors, so the cashier sees all of them at once.
 */
pub async fn quote(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<QuoteRequest>,
) -> Result<Json<ApiEnvelope<Quote>>, ApiError> {
    assert_can_sell(&mut tx, store_id).await?;
    let store = resolver::load_store_pricing(&mut tx, store_id).await?;
    let ctx = QuoteContext {
        tenant_id: tx.tenant_id,
        store_id,
        user_id: tx.user_id,
        customer_id: body.customer_id,
        at: Utc::now(),
    };
    let priced = quote::build(&mut tx, &ctx, &store, &body.lines, body.cart_discount.as_ref()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Cart priced.", priced)));
}

/// `GET /api/v1/stores/:id/catalog/snapshot?updated_since=` - the offline till's catalogue cache.
pub async fn snapshot(
    Path(store_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<ApiEnvelope<CatalogSnapshot>>, ApiError> {
    assert_can_sell(&mut tx, store_id).await?;
    let store = resolver::load_store_pricing(&mut tx, store_id).await?;
    let generated_at = Utc::now();
    let items = service::snapshot_items(&mut tx, &store, query.updated_since).await?;
    let price_rules = sqlx::query_as::<_, SnapshotPriceRule>(
        "SELECT pr.id, pr.product_id, pr.store_id, pr.customer_group_id, pr.min_quantity,
                pr.unit_price_amount_minor, pr.priority, pr.starts_at, pr.ends_at
         FROM atlas.price_rules pr
         JOIN atlas.product_store_listings psl ON psl.product_id = pr.product_id AND psl.store_id = $1 AND psl.is_active
         WHERE (pr.store_id IS NULL OR pr.store_id = $1)
           AND (pr.ends_at IS NULL OR pr.ends_at > now())
           AND ($2::timestamptz IS NULL OR pr.updated_at > $2)
         ORDER BY pr.product_id, pr.priority DESC",
    )
    .bind(store_id)
    .bind(query.updated_since)
    .fetch_all(&mut *tx)
    .await?;
    let tenant_id = tx.tenant_id;
    let tax_rates = sqlx::query_as::<_, TaxRate>(
        "SELECT id, name, rate_basis_points, is_active FROM atlas.tax_rates WHERE tenant_id = $1 ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(
        "Catalog snapshot generated.",
        CatalogSnapshot {
            store_id,
            currency: store.currency,
            prices_include_tax: store.prices_include_tax,
            cash_rounding_increment_minor: store.cash_rounding_increment_minor,
            generated_at,
            items,
            price_rules,
            tax_rates,
        },
    )));
}
