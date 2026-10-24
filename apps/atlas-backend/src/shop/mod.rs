/*!
 * The online storefront: the public shop API under `/shop/{handle}`, staff setup and order handling, promotions,
 * and the payment and courier callbacks. Third parties are reached only through `crate::integrations`.
 */
mod auth;
mod cart;
mod catalog;
mod checkout;
mod context;
mod loyalty;
mod orders;
mod pricing;
mod promotions;
mod settings;
mod staff;

use axum::routing::{get, post, put};
use axum::Router;

use crate::state::AppState;

pub use checkout::PAYMENT_SIGNATURE_HEADER;
pub use staff::COURIER_SIGNATURE_HEADER;

pub fn shop_router() -> Router<AppState> {
    return Router::new()
        .route("/{handle}", get(catalog::shop_info))
        .route("/{handle}/products", get(catalog::list_products))
        .route("/{handle}/products/{product_id}", get(catalog::show_product))
        .route("/{handle}/products/{product_id}/recommendations", get(catalog::recommendations))
        .route("/{handle}/categories", get(catalog::list_categories))
        .route("/{handle}/auth/signup", post(auth::signup))
        .route("/{handle}/auth/login", post(auth::login))
        .route("/{handle}/auth/social", post(auth::social))
        .route("/{handle}/account", get(auth::me))
        .route("/{handle}/account/loyalty", get(loyalty::account))
        .route("/{handle}/cart", get(cart::get_cart))
        .route("/{handle}/cart/items", post(cart::add_item))
        .route("/{handle}/cart/items/{product_id}", axum::routing::patch(cart::set_quantity).delete(cart::remove_item))
        .route("/{handle}/cart/items/{product_id}/move-to-wishlist", post(cart::move_to_wishlist))
        .route("/{handle}/wishlist", get(cart::get_wishlist))
        .route("/{handle}/wishlist/{product_id}", put(cart::add_wish).delete(cart::remove_wish))
        .route("/{handle}/wishlist/{product_id}/move-to-cart", post(cart::move_to_cart))
        .route("/{handle}/checkout/quote", post(checkout::quote))
        .route("/{handle}/checkout", post(checkout::place_order))
        .route("/{handle}/orders", get(checkout::my_orders))
        .route("/{handle}/orders/{order_id}", get(checkout::show_order))
        .route("/{handle}/orders/{order_id}/invoice", get(checkout::invoice))
        .route("/{handle}/orders/{order_id}/tracking", get(checkout::tracking))
        .route("/{handle}/orders/{order_id}/payments", post(checkout::retry_payment));
}

pub fn storefronts_router() -> Router<AppState> {
    return Router::new()
        .route("/{store_id}", get(settings::get_storefront).put(settings::upsert_storefront))
        .route("/{store_id}/shipping-zones", get(settings::list_zones).post(settings::create_zone))
        .route("/{store_id}/shipping-zones/{zone_id}", axum::routing::patch(settings::update_zone));
}

pub fn promotions_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(promotions::list_promotions).post(promotions::create_promotion))
        .route("/{id}", get(promotions::show_promotion).patch(promotions::update_promotion));
}

pub fn orders_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(staff::list_orders))
        .route("/{id}", get(staff::show_order))
        .route("/{id}/process", post(staff::process))
        .route("/{id}/ship", post(staff::ship))
        .route("/{id}/deliver", post(staff::deliver))
        .route("/{id}/cancel", post(staff::cancel))
        .route("/{id}/collect-payment", post(staff::collect_payment));
}

pub fn webhooks_router() -> Router<AppState> {
    return Router::new()
        .route("/payments", post(checkout::payment_webhook))
        .route("/couriers", post(staff::courier_webhook));
}
