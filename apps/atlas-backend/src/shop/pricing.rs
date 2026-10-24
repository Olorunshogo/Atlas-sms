/*!
 * Prices an online order on the server with the till's own resolver and engine, so tax and rounding can never
 * disagree with a sale. Order of the breakdown (FR-11.2): subtotal, discounts, shipping, tax, total. Promotions and
 * redeemed points are line and cart discounts handed to the engine, so tax is worked out on what is actually paid.
 */
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::pricing::engine::{self, Discount, LineInput};
use crate::pricing::resolver;

use super::catalog;
use super::context::Storefront;
use super::loyalty;
use super::promotions::{self, AppliedPromotion, PromoLine, Shopper};
use super::settings::{ShippingZone, ZONE_COLUMNS};

#[derive(Debug, Clone, Serialize)]
pub struct PricedLine {
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    pub unit_price_minor: i64,
    pub discount_minor: i64,
    pub net_minor: i64,
    pub tax_rate_basis_points: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub available: Decimal,
    #[serde(skip)]
    pub unit_cost_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PricedOrder {
    pub currency: String,
    pub prices_include_tax: bool,
    pub lines: Vec<PricedLine>,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub shipping_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub promotions: Vec<AppliedPromotion>,
    pub coupon_error: Option<String>,
    pub points_redeemed: i64,
    pub points_value_minor: i64,
    pub weight_kg: Decimal,
    pub shipping_zone_id: Uuid,
}

pub struct PriceRequest<'a> {
    pub shopper_id: Option<Uuid>,
    pub email: Option<&'a str>,
    pub lines: &'a [(Uuid, Decimal)],
    pub coupon_code: Option<&'a str>,
    pub shipping_zone_id: Uuid,
    pub points: i64,
}

pub async fn zone(conn: &mut PgConnection, store_id: Uuid, zone_id: Uuid) -> Result<ShippingZone, ApiError> {
    return sqlx::query_as::<_, ShippingZone>(&format!(
        "SELECT {ZONE_COLUMNS} FROM atlas.shipping_zones WHERE id = $1 AND store_id = $2 AND is_active"
    ))
    .bind(zone_id)
    .bind(store_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request_with_code("That shipping zone isn't available.", "SHIPPING_ZONE_INVALID"));
}

pub async fn price(conn: &mut PgConnection, storefront: &Storefront, request: &PriceRequest<'_>) -> Result<PricedOrder, ApiError> {
    if request.lines.is_empty() {
        return Err(ApiError::bad_request_with_code("The cart is empty.", "CART_EMPTY"));
    }
    let store = resolver::load_store_pricing(conn, storefront.store_id).await?;
    let group = catalog::customer_group(conn, request.shopper_id).await?;
    let shipping_zone = zone(conn, storefront.store_id, request.shipping_zone_id).await?;

    let mut products = Vec::with_capacity(request.lines.len());
    let mut promo_lines = Vec::with_capacity(request.lines.len());
    let mut weight = Decimal::ZERO;
    for (product_id, quantity) in request.lines {
        let product = catalog::sellable(conn, storefront.store_id, *product_id)
            .await?
            .ok_or_else(|| ApiError::conflict_with_code("Something in the cart is no longer sold here.", codes::PRODUCT_NOT_SOLD_HERE))?;
        let unit_price = catalog::regular_price(conn, storefront.store_id, *product_id, group, *quantity)
            .await?
            .ok_or_else(|| ApiError::conflict_with_code(format!("{} has no price here.", product.name), codes::PRODUCT_NOT_PRICED))?;
        let (_, tax_bp) = resolver::effective_tax_rate(conn, product.tax_rate_id, &store).await?;
        let available = catalog::available(conn, storefront.store_id, *product_id).await?.max(Decimal::ZERO);
        weight += catalog::weight_kg(&product.attributes) * *quantity;
        promo_lines.push(PromoLine { product_id: *product_id, category_id: product.category_id, quantity: *quantity, unit_price_minor: unit_price });
        products.push((product, unit_price, tax_bp, available, *quantity));
    }

    let shipping_before = shipping_zone.base_fee_minor + engine::round_minor(Decimal::from(shipping_zone.per_kg_fee_minor) * weight);
    let shopper = Shopper { shopper_id: request.shopper_id, email: request.email, customer_group_id: group };
    let outcome = promotions::evaluate(conn, storefront.store_id, &promo_lines, &shopper, request.coupon_code, shipping_before).await?;

    // Points are a cart discount on the goods left after promotions
    let after_promotions: i64 = promo_lines
        .iter()
        .zip(&outcome.line_discounts)
        .map(|(line, discount)| engine::round_minor(Decimal::from(line.unit_price_minor) * line.quantity) - discount)
        .sum();
    let points_value_minor = if request.points > 0 {
        let shopper_id = request.shopper_id.ok_or_else(|| ApiError::bad_request_with_code("Sign in to use loyalty points.", "LOYALTY_REDEMPTION_INVALID"))?;
        if storefront.loyalty_minor_per_point.is_none() {
            return Err(ApiError::bad_request_with_code("This shop doesn't run loyalty points.", "LOYALTY_REDEMPTION_INVALID"));
        }
        let (available_points, _) = loyalty::balance(conn, shopper_id).await?;
        let cap = loyalty::max_redeemable(storefront, after_promotions);
        if request.points > available_points || request.points > cap {
            return Err(ApiError::bad_request_with_code(
                format!("You can use up to {} points on this order ({} available).", cap.min(available_points), available_points),
                "LOYALTY_REDEMPTION_INVALID",
            ));
        }
        request.points * storefront.loyalty_point_value_minor
    } else {
        0
    };

    let inputs: Vec<LineInput> = products
        .iter()
        .zip(&outcome.line_discounts)
        .map(|((_, unit_price, tax_bp, _, quantity), discount)| LineInput {
            unit_price_minor: *unit_price,
            quantity: *quantity,
            discount: (*discount > 0).then_some(Discount::Amount(*discount)),
            tax_rate_basis_points: *tax_bp,
        })
        .collect();
    let cart_discount = (points_value_minor > 0).then_some(Discount::Amount(points_value_minor));
    let totals = engine::price_cart(&inputs, cart_discount, store.prices_include_tax);

    let lines = products
        .into_iter()
        .zip(&totals.lines)
        .map(|((product, unit_price, tax_bp, available, quantity), line)| PricedLine {
            product_id: product.id,
            product_name: product.name,
            sku: product.sku,
            quantity,
            unit_price_minor: unit_price,
            discount_minor: line.line_discount_minor + line.allocated_cart_discount_minor,
            net_minor: line.net_minor,
            tax_rate_basis_points: tax_bp,
            tax_minor: line.tax_minor,
            total_minor: line.total_minor,
            available,
            unit_cost_minor: product.unit_cost_minor,
        })
        .collect();
    let shipping_minor = (shipping_before - outcome.shipping_discount_minor).max(0);
    return Ok(PricedOrder {
        currency: store.currency,
        prices_include_tax: store.prices_include_tax,
        lines,
        subtotal_minor: totals.subtotal_minor,
        discount_minor: totals.discount_minor,
        shipping_minor,
        tax_minor: totals.tax_minor,
        total_minor: totals.total_minor + shipping_minor,
        promotions: outcome.applied,
        coupon_error: outcome.coupon_error,
        points_redeemed: request.points.max(0),
        points_value_minor,
        weight_kg: weight,
        shipping_zone_id: shipping_zone.id,
    });
}
