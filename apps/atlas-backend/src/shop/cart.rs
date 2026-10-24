/*!
 * Carts and wishlists. A shopper's cart lives on the server and follows them across devices; a guest's is found by
 * an opaque `x-cart-token` (stored hashed) and kept `guest_cart_days`. Nothing is reserved while it sits in a cart;
 * the view says when a price moved or an item ran out since it was added. Signing in merges the guest cart in.
 */
use axum::extract::Path;
use axum::Json;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::ApiError;
use crate::pricing::engine;
use crate::response::ApiEnvelope;

use super::catalog;
use super::context::ShopTx;

#[derive(Debug, Serialize)]
pub struct CartItem {
    pub product_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    pub unit_price_minor: i64,
    pub added_unit_price_minor: i64,
    pub price_changed: bool,
    pub available: Decimal,
    pub in_stock: bool,
    pub line_total_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct CartView {
    /// Only for a guest: send it back as `x-cart-token`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cart_token: Option<String>,
    pub items: Vec<CartItem>,
    pub subtotal_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    pub product_id: Uuid,
    pub quantity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct SetQuantityRequest {
    pub quantity: Decimal,
}

// === Finding the cart

async fn find_cart(tx: &mut ShopTx) -> Result<Option<Uuid>, ApiError> {
    let store_id = tx.store_id;
    if let Some(shopper_id) = tx.shopper_id {
        return sqlx::query_scalar("SELECT id FROM atlas.carts WHERE store_id = $1 AND shopper_id = $2")
            .bind(store_id)
            .bind(shopper_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(ApiError::from);
    }
    let Some(token) = tx.cart_token.clone() else { return Ok(None) };
    let found: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, expires_at < now() FROM atlas.carts WHERE store_id = $1 AND guest_token_hash = $2",
    )
    .bind(store_id)
    .bind(crypto::hash_lookup_token(&token))
    .fetch_optional(&mut **tx)
    .await?;
    return match found {
        Some((id, true)) => {
            sqlx::query("DELETE FROM atlas.carts WHERE id = $1").bind(id).execute(&mut **tx).await?;
            Ok(None)
        }
        Some((id, false)) => Ok(Some(id)),
        None => Ok(None),
    };
}

/// The cart to write to, made if needed. A new guest cart comes back with its token.
async fn cart_for_write(tx: &mut ShopTx) -> Result<(Uuid, Option<String>), ApiError> {
    let expires = Utc::now() + Duration::days(i64::from(tx.storefront.guest_cart_days));
    if let Some(id) = find_cart(tx).await? {
        if tx.shopper_id.is_none() {
            sqlx::query("UPDATE atlas.carts SET expires_at = $2, updated_at = now() WHERE id = $1")
                .bind(id)
                .bind(expires)
                .execute(&mut **tx)
                .await?;
        }
        return Ok((id, None));
    }
    let (tenant_id, store_id) = (tx.tenant_id, tx.store_id);
    let id = Uuid::now_v7();
    if let Some(shopper_id) = tx.shopper_id {
        sqlx::query("INSERT INTO atlas.carts (id, tenant_id, store_id, shopper_id) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(tenant_id)
            .bind(store_id)
            .bind(shopper_id)
            .execute(&mut **tx)
            .await?;
        return Ok((id, None));
    }
    let token = crypto::generate_opaque_token();
    sqlx::query("INSERT INTO atlas.carts (id, tenant_id, store_id, guest_token_hash, expires_at) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(tenant_id)
        .bind(store_id)
        .bind(crypto::hash_lookup_token(&token))
        .bind(expires)
        .execute(&mut **tx)
        .await?;
    tx.cart_token = Some(token.clone());
    return Ok((id, Some(token)));
}

/// `(product_id, quantity)` for every line in the caller's cart.
pub async fn lines(tx: &mut ShopTx) -> Result<Vec<(Uuid, Decimal)>, ApiError> {
    let Some(cart_id) = find_cart(tx).await? else { return Ok(Vec::new()) };
    return sqlx::query_as("SELECT product_id, quantity FROM atlas.cart_items WHERE cart_id = $1 ORDER BY added_at, product_id")
        .bind(cart_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(ApiError::from);
}

pub async fn clear(tx: &mut ShopTx) -> Result<(), ApiError> {
    if let Some(cart_id) = find_cart(tx).await? {
        sqlx::query("DELETE FROM atlas.cart_items WHERE cart_id = $1").bind(cart_id).execute(&mut **tx).await?;
    }
    return Ok(());
}

async fn current_price(tx: &mut ShopTx, product_id: Uuid, quantity: Decimal) -> Result<i64, ApiError> {
    let store_id = tx.store_id;
    let shopper_id = tx.shopper_id;
    catalog::sellable(tx, store_id, product_id).await?.ok_or_else(|| ApiError::not_found("Product not found."))?;
    let group = catalog::customer_group(tx, shopper_id).await?;
    return catalog::regular_price(tx, store_id, product_id, group, quantity)
        .await?
        .ok_or_else(|| ApiError::conflict_with_code("This product has no price here.", "PRODUCT_NOT_PRICED"));
}

async fn view(tx: &mut ShopTx, new_token: Option<String>) -> Result<CartView, ApiError> {
    let store_id = tx.store_id;
    let shopper_id = tx.shopper_id;
    let Some(cart_id) = find_cart(tx).await? else {
        return Ok(CartView { cart_token: new_token, items: Vec::new(), subtotal_minor: 0 });
    };
    let rows: Vec<(Uuid, Decimal, i64)> =
        sqlx::query_as("SELECT product_id, quantity, added_unit_price_minor FROM atlas.cart_items WHERE cart_id = $1 ORDER BY added_at, product_id")
            .bind(cart_id)
            .fetch_all(&mut **tx)
            .await?;
    let group = catalog::customer_group(tx, shopper_id).await?;
    let mut items = Vec::with_capacity(rows.len());
    for (product_id, quantity, added) in rows {
        let Some(product) = catalog::sellable(tx, store_id, product_id).await? else { continue };
        let Some(price) = catalog::regular_price(tx, store_id, product_id, group, quantity).await? else { continue };
        let available = catalog::available(tx, store_id, product_id).await?.max(Decimal::ZERO);
        items.push(CartItem {
            product_id,
            name: product.name,
            sku: product.sku,
            quantity,
            unit_price_minor: price,
            added_unit_price_minor: added,
            price_changed: price != added,
            in_stock: available >= quantity,
            available,
            line_total_minor: engine::round_minor(Decimal::from(price) * quantity),
        });
    }
    let subtotal_minor = items.iter().map(|i| i.line_total_minor).sum();
    return Ok(CartView { cart_token: new_token, items, subtotal_minor });
}

fn check_quantity(quantity: Decimal) -> Result<(), ApiError> {
    if quantity <= Decimal::ZERO {
        return Err(ApiError::bad_request_with_code("quantity must be positive", "VALIDATION_FAILED"));
    }
    return Ok(());
}

// === Handlers

/// `GET /api/v1/shop/:handle/cart`.
pub async fn get_cart(mut tx: ShopTx) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    let cart = view(&mut tx, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Cart fetched.", cart)));
}

/// `POST /api/v1/shop/:handle/cart/items`. Adds to what is already there.
pub async fn add_item(mut tx: ShopTx, Json(body): Json<AddItemRequest>) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    check_quantity(body.quantity)?;
    let price = current_price(&mut tx, body.product_id, body.quantity).await?;
    let (cart_id, token) = cart_for_write(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    sqlx::query(
        "INSERT INTO atlas.cart_items (cart_id, product_id, tenant_id, quantity, added_unit_price_minor)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (cart_id, product_id) DO UPDATE SET quantity = atlas.cart_items.quantity + EXCLUDED.quantity",
    )
    .bind(cart_id)
    .bind(body.product_id)
    .bind(tenant_id)
    .bind(body.quantity)
    .bind(price)
    .execute(&mut *tx)
    .await?;
    let cart = view(&mut tx, token).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Added to cart.", cart)));
}

/// `PATCH /api/v1/shop/:handle/cart/items/:product_id`.
pub async fn set_quantity(
    Path((_handle, product_id)): Path<(String, Uuid)>,
    mut tx: ShopTx,
    Json(body): Json<SetQuantityRequest>,
) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    check_quantity(body.quantity)?;
    let cart_id = find_cart(&mut tx).await?.ok_or_else(|| ApiError::not_found("That item isn't in the cart."))?;
    let updated = sqlx::query("UPDATE atlas.cart_items SET quantity = $3 WHERE cart_id = $1 AND product_id = $2")
        .bind(cart_id)
        .bind(product_id)
        .bind(body.quantity)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("That item isn't in the cart."));
    }
    let cart = view(&mut tx, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Cart updated.", cart)));
}

/// `DELETE /api/v1/shop/:handle/cart/items/:product_id`.
pub async fn remove_item(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    if let Some(cart_id) = find_cart(&mut tx).await? {
        sqlx::query("DELETE FROM atlas.cart_items WHERE cart_id = $1 AND product_id = $2")
            .bind(cart_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
    }
    let cart = view(&mut tx, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Removed from cart.", cart)));
}

/// `POST /api/v1/shop/:handle/cart/items/:product_id/move-to-wishlist`. Needs a signed-in shopper.
pub async fn move_to_wishlist(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    save_wish(&mut tx, shopper_id, product_id).await?;
    if let Some(cart_id) = find_cart(&mut tx).await? {
        sqlx::query("DELETE FROM atlas.cart_items WHERE cart_id = $1 AND product_id = $2")
            .bind(cart_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
    }
    let cart = view(&mut tx, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Moved to wishlist.", cart)));
}

/**
 * Folds a guest cart into the shopper's, adding quantities together up to what is in stock, then drops the guest
 * cart. Called on sign-up, log-in and social sign-in when the request carries `x-cart-token`.
 */
pub async fn merge_guest_cart(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, guest_token: &str, shopper_id: Uuid) -> Result<(), ApiError> {
    let guest: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.carts WHERE store_id = $1 AND guest_token_hash = $2 AND expires_at > now()")
        .bind(store_id)
        .bind(crypto::hash_lookup_token(guest_token))
        .fetch_optional(&mut *conn)
        .await?;
    let Some(guest_id) = guest else { return Ok(()) };
    let mine: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM atlas.carts WHERE store_id = $1 AND shopper_id = $2")
        .bind(store_id)
        .bind(shopper_id)
        .fetch_optional(&mut *conn)
        .await?
    {
        Some(id) => id,
        None => {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO atlas.carts (id, tenant_id, store_id, shopper_id) VALUES ($1, $2, $3, $4)")
                .bind(id)
                .bind(tenant_id)
                .bind(store_id)
                .bind(shopper_id)
                .execute(&mut *conn)
                .await?;
            id
        }
    };
    let items: Vec<(Uuid, Decimal, i64)> = sqlx::query_as("SELECT product_id, quantity, added_unit_price_minor FROM atlas.cart_items WHERE cart_id = $1")
        .bind(guest_id)
        .fetch_all(&mut *conn)
        .await?;
    for (product_id, quantity, added) in items {
        let existing: Decimal = sqlx::query_scalar("SELECT COALESCE((SELECT quantity FROM atlas.cart_items WHERE cart_id = $1 AND product_id = $2), 0)")
            .bind(mine)
            .bind(product_id)
            .fetch_one(&mut *conn)
            .await?;
        let in_stock = catalog::available(conn, store_id, product_id).await?.max(Decimal::ZERO);
        let merged = (existing + quantity).min(in_stock.max(existing));
        if merged <= Decimal::ZERO {
            continue;
        }
        sqlx::query(
            "INSERT INTO atlas.cart_items (cart_id, product_id, tenant_id, quantity, added_unit_price_minor)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (cart_id, product_id) DO UPDATE SET quantity = EXCLUDED.quantity",
        )
        .bind(mine)
        .bind(product_id)
        .bind(tenant_id)
        .bind(merged)
        .bind(added)
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query("DELETE FROM atlas.carts WHERE id = $1").bind(guest_id).execute(conn).await?;
    return Ok(());
}

// === Wishlist

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WishlistItem {
    pub product_id: Uuid,
    pub name: String,
    pub added_at: chrono::DateTime<Utc>,
}

async fn save_wish(tx: &mut ShopTx, shopper_id: Uuid, product_id: Uuid) -> Result<(), ApiError> {
    let (tenant_id, store_id) = (tx.tenant_id, tx.store_id);
    catalog::sellable(tx, store_id, product_id).await?.ok_or_else(|| ApiError::not_found("Product not found."))?;
    sqlx::query(
        "INSERT INTO atlas.wishlist_items (shopper_id, store_id, product_id, tenant_id) VALUES ($1, $2, $3, $4)
         ON CONFLICT DO NOTHING",
    )
    .bind(shopper_id)
    .bind(store_id)
    .bind(product_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await?;
    return Ok(());
}

async fn wishlist(tx: &mut ShopTx, shopper_id: Uuid) -> Result<Vec<WishlistItem>, ApiError> {
    let store_id = tx.store_id;
    return sqlx::query_as::<_, WishlistItem>(
        "SELECT w.product_id, p.name, w.added_at FROM atlas.wishlist_items w JOIN atlas.products p ON p.id = w.product_id
         WHERE w.shopper_id = $1 AND w.store_id = $2 ORDER BY w.added_at DESC",
    )
    .bind(shopper_id)
    .bind(store_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(ApiError::from);
}

/// `GET /api/v1/shop/:handle/wishlist`. Wishlists need an account.
pub async fn get_wishlist(mut tx: ShopTx) -> Result<Json<ApiEnvelope<Vec<WishlistItem>>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let items = wishlist(&mut tx, shopper_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Wishlist fetched.", items)));
}

/// `PUT /api/v1/shop/:handle/wishlist/:product_id`.
pub async fn add_wish(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<Vec<WishlistItem>>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    save_wish(&mut tx, shopper_id, product_id).await?;
    let items = wishlist(&mut tx, shopper_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Saved to wishlist.", items)));
}

/// `DELETE /api/v1/shop/:handle/wishlist/:product_id`.
pub async fn remove_wish(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<Vec<WishlistItem>>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let store_id = tx.store_id;
    sqlx::query("DELETE FROM atlas.wishlist_items WHERE shopper_id = $1 AND store_id = $2 AND product_id = $3")
        .bind(shopper_id)
        .bind(store_id)
        .bind(product_id)
        .execute(&mut *tx)
        .await?;
    let items = wishlist(&mut tx, shopper_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Removed from wishlist.", items)));
}

/// `POST /api/v1/shop/:handle/wishlist/:product_id/move-to-cart`. One unit goes into the cart.
pub async fn move_to_cart(Path((_handle, product_id)): Path<(String, Uuid)>, mut tx: ShopTx) -> Result<Json<ApiEnvelope<CartView>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let price = current_price(&mut tx, product_id, Decimal::ONE).await?;
    let (cart_id, _) = cart_for_write(&mut tx).await?;
    let (tenant_id, store_id) = (tx.tenant_id, tx.store_id);
    sqlx::query(
        "INSERT INTO atlas.cart_items (cart_id, product_id, tenant_id, quantity, added_unit_price_minor) VALUES ($1, $2, $3, 1, $4)
         ON CONFLICT (cart_id, product_id) DO NOTHING",
    )
    .bind(cart_id)
    .bind(product_id)
    .bind(tenant_id)
    .bind(price)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM atlas.wishlist_items WHERE shopper_id = $1 AND store_id = $2 AND product_id = $3")
        .bind(shopper_id)
        .bind(store_id)
        .bind(product_id)
        .execute(&mut *tx)
        .await?;
    let cart = view(&mut tx, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Moved to cart.", cart)));
}
