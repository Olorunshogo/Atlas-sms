/*!
 * Promotions: coupons, automatic cart promotions, flash sales and bundles, each with eligibility, a window, and
 * total and per-customer usage limits. Staff manage them with `promotion.manage`.
 *
 * Stacking: at most one coupon; an automatic promotion stacks with a coupon or another automatic one only when both
 * are stackable; flash-sale lines are left out of other promotions unless the flash sale is stackable. Of the
 * combinations allowed (any one alone, or every stackable one together) the customer gets the best.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::pricing::engine;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::required_text;

const PROMOTION_COLUMNS: &str = "id, store_id, name, kind::text AS kind, discount::text AS discount, discount_value, coupon_code,
     product_ids, category_ids, bundle_product_ids, customer_group_id, first_order_only, min_spend_minor, min_quantity,
     flash_price_minor, flash_stock_cap, stackable, starts_at, ends_at, total_usage_limit, per_customer_limit, is_active, created_at";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Promotion {
    pub id: Uuid,
    pub store_id: Option<Uuid>,
    pub name: String,
    pub kind: String,
    pub discount: String,
    pub discount_value: i64,
    pub coupon_code: Option<String>,
    pub product_ids: Vec<Uuid>,
    pub category_ids: Vec<Uuid>,
    pub bundle_product_ids: Vec<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub first_order_only: bool,
    pub min_spend_minor: Option<i64>,
    pub min_quantity: Option<Decimal>,
    pub flash_price_minor: Option<i64>,
    pub flash_stock_cap: Option<Decimal>,
    pub stackable: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub total_usage_limit: Option<i32>,
    pub per_customer_limit: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl Promotion {
    fn covers(&self, product_id: Uuid, category_id: Option<Uuid>) -> bool {
        if self.product_ids.is_empty() && self.category_ids.is_empty() {
            return true;
        }
        return self.product_ids.contains(&product_id) || category_id.is_some_and(|c| self.category_ids.contains(&c));
    }
}

// === Evaluation

#[derive(Debug, Clone)]
pub struct PromoLine {
    pub product_id: Uuid,
    pub category_id: Option<Uuid>,
    pub quantity: Decimal,
    pub unit_price_minor: i64,
}

pub struct Shopper<'a> {
    pub shopper_id: Option<Uuid>,
    pub email: Option<&'a str>,
    pub customer_group_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedPromotion {
    pub promotion_id: Uuid,
    pub name: String,
    pub kind: String,
    pub discount_minor: i64,
    /// Units sold at a flash price.
    #[serde(skip_serializing_if = "Decimal::is_zero")]
    pub units: Decimal,
}

#[derive(Debug, Clone, Default)]
pub struct PromotionOutcome {
    pub line_discounts: Vec<i64>,
    pub shipping_discount_minor: i64,
    pub applied: Vec<AppliedPromotion>,
    /// Why the coupon given didn't apply, when it didn't.
    pub coupon_error: Option<String>,
}

async fn live(conn: &mut PgConnection, store_id: Uuid, shopper: &Shopper<'_>) -> Result<Vec<Promotion>, ApiError> {
    let candidates = sqlx::query_as::<_, Promotion>(&format!(
        "SELECT {PROMOTION_COLUMNS} FROM atlas.promotions p
         WHERE p.is_active AND (p.store_id IS NULL OR p.store_id = $1)
           AND (p.starts_at IS NULL OR p.starts_at <= now()) AND (p.ends_at IS NULL OR p.ends_at > now())
           AND (p.customer_group_id IS NULL OR p.customer_group_id = $2)
           AND (p.total_usage_limit IS NULL
                OR (SELECT COUNT(*) FROM atlas.promotion_redemptions r WHERE r.promotion_id = p.id) < p.total_usage_limit)
           AND (p.per_customer_limit IS NULL
                OR (SELECT COUNT(*) FROM atlas.promotion_redemptions r
                    WHERE r.promotion_id = p.id AND (r.shopper_id = $3 OR lower(r.email) = lower($4))) < p.per_customer_limit)
         ORDER BY p.created_at"
    ))
    .bind(store_id)
    .bind(shopper.customer_group_id)
    .bind(shopper.shopper_id)
    .bind(shopper.email)
    .fetch_all(&mut *conn)
    .await?;
    return Ok(candidates);
}

async fn is_first_order(conn: &mut PgConnection, shopper: &Shopper<'_>) -> Result<bool, ApiError> {
    if shopper.shopper_id.is_none() && shopper.email.is_none() {
        return Ok(true);
    }
    let earlier: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atlas.shop_orders
         WHERE status NOT IN ('cancelled', 'expired') AND (shopper_id = $1 OR lower(email) = lower($2))",
    )
    .bind(shopper.shopper_id)
    .bind(shopper.email)
    .fetch_one(conn)
    .await?;
    return Ok(earlier == 0);
}

async fn flash_units_left(conn: &mut PgConnection, promotion: &Promotion) -> Result<Decimal, ApiError> {
    let used: Decimal = sqlx::query_scalar("SELECT COALESCE(SUM(units), 0) FROM atlas.promotion_redemptions WHERE promotion_id = $1")
        .bind(promotion.id)
        .fetch_one(conn)
        .await?;
    return Ok(promotion.flash_stock_cap.map(|cap| (cap - used).max(Decimal::ZERO)).unwrap_or(Decimal::MAX));
}

/// The flash price for a product right now, if a flash sale with units left covers it.
pub async fn flash_price(conn: &mut PgConnection, store_id: Uuid, product_id: Uuid) -> Result<Option<i64>, ApiError> {
    let category: Option<Option<Uuid>> = sqlx::query_scalar("SELECT category_id FROM atlas.products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&mut *conn)
        .await?;
    let shopper = Shopper { shopper_id: None, email: None, customer_group_id: None };
    let mut best: Option<i64> = None;
    for promotion in live(conn, store_id, &shopper).await? {
        if promotion.kind != "flash_sale" || promotion.customer_group_id.is_some() || !promotion.covers(product_id, category.flatten()) {
            continue;
        }
        if flash_units_left(conn, &promotion).await? <= Decimal::ZERO {
            continue;
        }
        if let Some(price) = promotion.flash_price_minor {
            best = Some(best.map_or(price, |b| b.min(price)));
        }
    }
    return Ok(best);
}

struct Candidate {
    promotion: Promotion,
    lines: Vec<usize>,
    discount_minor: i64,
    shipping_minor: i64,
}

fn gross(line: &PromoLine) -> i64 {
    return engine::round_minor(Decimal::from(line.unit_price_minor) * line.quantity);
}

/// Works out which promotions apply to a cart and how much each line saves. `shipping_minor` is the fee before promotions.
pub async fn evaluate(
    conn: &mut PgConnection,
    store_id: Uuid,
    lines: &[PromoLine],
    shopper: &Shopper<'_>,
    coupon_code: Option<&str>,
    shipping_minor: i64,
) -> Result<PromotionOutcome, ApiError> {
    let mut outcome = PromotionOutcome { line_discounts: vec![0; lines.len()], ..PromotionOutcome::default() };
    let first_order = is_first_order(conn, shopper).await?;
    let promotions: Vec<Promotion> = live(conn, store_id, shopper)
        .await?
        .into_iter()
        .filter(|p| !p.first_order_only || first_order)
        .collect();

    // Flash sales first: they change the price itself, up to the units left in the cap
    let mut sale_priced: Vec<bool> = vec![false; lines.len()];
    for promotion in promotions.iter().filter(|p| p.kind == "flash_sale") {
        let Some(flash) = promotion.flash_price_minor else { continue };
        let mut left = flash_units_left(conn, promotion).await?;
        let mut saved: i64 = 0;
        let mut units = Decimal::ZERO;
        for (i, line) in lines.iter().enumerate() {
            if left <= Decimal::ZERO || flash >= line.unit_price_minor || !promotion.covers(line.product_id, line.category_id) || outcome.line_discounts[i] > 0 {
                continue;
            }
            let take = line.quantity.min(left);
            let discount = engine::round_minor(Decimal::from(line.unit_price_minor - flash) * take);
            outcome.line_discounts[i] += discount;
            if !promotion.stackable {
                sale_priced[i] = true;
            }
            left -= take;
            saved += discount;
            units += take;
        }
        if saved > 0 {
            outcome.applied.push(AppliedPromotion {
                promotion_id: promotion.id,
                name: promotion.name.clone(),
                kind: promotion.kind.clone(),
                discount_minor: saved,
                units,
            });
        }
    }

    let code = coupon_code.map(str::trim).filter(|c| !c.is_empty()).map(str::to_uppercase);
    let mut coupon_seen = false;
    let mut candidates: Vec<Candidate> = Vec::new();
    for promotion in promotions.into_iter().filter(|p| p.kind != "flash_sale") {
        if promotion.kind == "coupon" {
            if code.as_deref() != promotion.coupon_code.as_deref().map(str::to_uppercase).as_deref() {
                continue;
            }
            coupon_seen = true;
        }
        let eligible: Vec<usize> = (0..lines.len())
            .filter(|&i| (!sale_priced[i] || promotion.stackable) && promotion.covers(lines[i].product_id, lines[i].category_id))
            .filter(|&i| promotion.kind != "bundle" || promotion.bundle_product_ids.contains(&lines[i].product_id))
            .collect();
        if promotion.kind == "bundle" && !promotion.bundle_product_ids.iter().all(|id| eligible.iter().any(|&i| lines[i].product_id == *id)) {
            continue;
        }
        let base: i64 = eligible.iter().map(|&i| gross(&lines[i]) - outcome.line_discounts[i]).sum();
        let quantity: Decimal = eligible.iter().map(|&i| lines[i].quantity).sum();
        if eligible.is_empty() || promotion.min_spend_minor.is_some_and(|min| base < min) || promotion.min_quantity.is_some_and(|min| quantity < min) {
            if promotion.kind == "coupon" {
                outcome.coupon_error = Some(match promotion.min_spend_minor {
                    Some(min) if base < min => format!("This coupon needs a spend of {} on eligible items.", min),
                    _ => "Nothing in the cart qualifies for this coupon.".to_string(),
                });
            }
            continue;
        }
        let (discount_minor, shipping_saved) = match promotion.discount.as_str() {
            "percentage" => (engine::round_minor(Decimal::from(base) * Decimal::from(promotion.discount_value) / Decimal::from(engine::BASIS_POINTS)), 0),
            "fixed_amount" => (promotion.discount_value.min(base), 0),
            _ => (0, shipping_minor),
        };
        if discount_minor + shipping_saved > 0 {
            candidates.push(Candidate { promotion, lines: eligible, discount_minor, shipping_minor: shipping_saved });
        }
    }
    if code.is_some() && !coupon_seen && outcome.coupon_error.is_none() {
        outcome.coupon_error = Some("That coupon isn't valid here, has ended, or has been used up.".to_string());
    }

    // Each alone, or every stackable one together; the customer gets whichever saves the most
    let value = |c: &Candidate| -> i64 { c.discount_minor + c.shipping_minor };
    let best_single = candidates.iter().enumerate().max_by_key(|(_, c)| value(c)).map(|(i, c)| (vec![i], value(c)));
    let stack: Vec<usize> = candidates.iter().enumerate().filter(|(_, c)| c.promotion.stackable).map(|(i, _)| i).collect();
    let stack_value: i64 = stack.iter().map(|&i| value(&candidates[i])).sum();
    let chosen: Vec<usize> = match best_single {
        Some((single, single_value)) if single_value >= stack_value => single,
        _ => stack,
    };
    if outcome.coupon_error.is_none() && code.is_some() && !chosen.iter().any(|&i| candidates[i].promotion.kind == "coupon") {
        outcome.coupon_error = Some("A better promotion already applies, and this coupon can't be combined with it.".to_string());
    }

    for &i in &chosen {
        let candidate = &candidates[i];
        // Never take a line below zero when two promotions share it
        let room: Vec<i64> = candidate.lines.iter().map(|&l| (gross(&lines[l]) - outcome.line_discounts[l]).max(0)).collect();
        let total_room: i64 = room.iter().sum();
        let discount = candidate.discount_minor.min(total_room);
        let shares = engine::allocate(discount, &room);
        for (share, &l) in shares.iter().zip(&candidate.lines) {
            outcome.line_discounts[l] += share;
        }
        outcome.shipping_discount_minor = (outcome.shipping_discount_minor + candidate.shipping_minor).min(shipping_minor);
        outcome.applied.push(AppliedPromotion {
            promotion_id: candidate.promotion.id,
            name: candidate.promotion.name.clone(),
            kind: candidate.promotion.kind.clone(),
            discount_minor: discount + candidate.shipping_minor,
            units: Decimal::ZERO,
        });
    }
    return Ok(outcome);
}

/// Records what an order took from each promotion, so later usage limits and flash caps count it.
pub async fn redeem(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    order_id: Uuid,
    shopper_id: Option<Uuid>,
    email: &str,
    applied: &[AppliedPromotion],
) -> Result<(), ApiError> {
    for promotion in applied {
        sqlx::query(
            "INSERT INTO atlas.promotion_redemptions (id, tenant_id, promotion_id, order_id, shopper_id, email, discount_minor, units)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(promotion.promotion_id)
        .bind(order_id)
        .bind(shopper_id)
        .bind(email)
        .bind(promotion.discount_minor)
        .bind(promotion.units)
        .execute(&mut *conn)
        .await?;
    }
    return Ok(());
}

/// A cancelled or expired order gives its uses back.
pub async fn release(conn: &mut PgConnection, order_id: Uuid) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM atlas.promotion_redemptions WHERE order_id = $1")
        .bind(order_id)
        .execute(conn)
        .await?;
    return Ok(());
}

// === Staff management

#[derive(Debug, Deserialize)]
pub struct PromotionRequest {
    pub store_id: Option<Uuid>,
    pub name: String,
    pub kind: String,
    pub discount: String,
    #[serde(default)]
    pub discount_value: i64,
    pub coupon_code: Option<String>,
    #[serde(default)]
    pub product_ids: Vec<Uuid>,
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub bundle_product_ids: Vec<Uuid>,
    pub customer_group_id: Option<Uuid>,
    #[serde(default)]
    pub first_order_only: bool,
    pub min_spend_minor: Option<i64>,
    pub min_quantity: Option<Decimal>,
    pub flash_price_minor: Option<i64>,
    pub flash_stock_cap: Option<Decimal>,
    #[serde(default)]
    pub stackable: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub total_usage_limit: Option<i32>,
    pub per_customer_limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePromotionRequest {
    pub name: Option<String>,
    pub discount_value: Option<i64>,
    pub stackable: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub total_usage_limit: Option<i32>,
    pub per_customer_limit: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListPromotionsQuery {
    pub store_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PromotionDetail {
    #[serde(flatten)]
    pub promotion: Promotion,
    pub used_count: i64,
    pub units_sold: Decimal,
}

async fn assert_manage(tx: &mut TenantTx, store_id: Option<Uuid>) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, store_id, "promotion.manage").await;
}

fn validate(body: &PromotionRequest) -> Result<(), ApiError> {
    let fail = |message: &str| Err(ApiError::bad_request_with_code(message.to_string(), "VALIDATION_FAILED"));
    if !["coupon", "automatic", "flash_sale", "bundle"].contains(&body.kind.as_str()) {
        return fail("kind must be coupon, automatic, flash_sale or bundle");
    }
    if !["percentage", "fixed_amount", "free_shipping"].contains(&body.discount.as_str()) {
        return fail("discount must be percentage, fixed_amount or free_shipping");
    }
    if body.discount_value < 0 || (body.discount == "percentage" && body.discount_value > engine::BASIS_POINTS) {
        return fail("a percentage is 0 to 10000 basis points and an amount can't be negative");
    }
    if (body.kind == "coupon") != body.coupon_code.as_deref().is_some_and(|c| !c.trim().is_empty()) {
        return fail("a coupon needs a coupon_code, and only a coupon has one");
    }
    if body.kind == "flash_sale" && (body.flash_price_minor.is_none() || (body.product_ids.is_empty() && body.category_ids.is_empty())) {
        return fail("a flash sale needs flash_price_minor and the products or categories it covers");
    }
    if body.kind != "flash_sale" && body.flash_price_minor.is_some() {
        return fail("only a flash sale has a flash price");
    }
    if body.kind == "bundle" && body.bundle_product_ids.len() < 2 {
        return fail("a bundle needs at least two bundle_product_ids");
    }
    if let (Some(start), Some(end)) = (body.starts_at, body.ends_at) {
        if end <= start {
            return fail("ends_at must be after starts_at");
        }
    }
    return Ok(());
}

async fn detail(conn: &mut PgConnection, promotion_id: Uuid) -> Result<PromotionDetail, ApiError> {
    let promotion = sqlx::query_as::<_, Promotion>(&format!("SELECT {PROMOTION_COLUMNS} FROM atlas.promotions WHERE id = $1"))
        .bind(promotion_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Promotion not found."))?;
    let (used_count, units_sold): (i64, Decimal) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(units), 0) FROM atlas.promotion_redemptions WHERE promotion_id = $1")
            .bind(promotion_id)
            .fetch_one(conn)
            .await?;
    return Ok(PromotionDetail { promotion, used_count, units_sold });
}

/// `GET /api/v1/promotions?store_id=`.
pub async fn list_promotions(mut tx: TenantTx, Query(query): Query<ListPromotionsQuery>) -> Result<Json<ApiEnvelope<Vec<Promotion>>>, ApiError> {
    assert_manage(&mut tx, query.store_id).await?;
    let tenant_id = tx.tenant_id;
    let promotions = sqlx::query_as::<_, Promotion>(&format!(
        "SELECT {PROMOTION_COLUMNS} FROM atlas.promotions
         WHERE tenant_id = $1 AND ($2::uuid IS NULL OR store_id IS NULL OR store_id = $2)
         ORDER BY created_at DESC"
    ))
    .bind(tenant_id)
    .bind(query.store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Promotions fetched.", promotions)));
}

/// `POST /api/v1/promotions`. `store_id: null` runs it at every store.
pub async fn create_promotion(mut tx: TenantTx, Json(body): Json<PromotionRequest>) -> Result<Json<ApiEnvelope<PromotionDetail>>, ApiError> {
    assert_manage(&mut tx, body.store_id).await?;
    validate(&body)?;
    let name = required_text("name", &body.name, 200)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.promotions
            (id, tenant_id, store_id, name, kind, discount, discount_value, coupon_code, product_ids, category_ids, bundle_product_ids,
             customer_group_id, first_order_only, min_spend_minor, min_quantity, flash_price_minor, flash_stock_cap, stackable,
             starts_at, ends_at, total_usage_limit, per_customer_limit)
         VALUES ($1, $2, $3, $4, $5::atlas.promotion_kind, $6::atlas.promotion_discount, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                 $16, $17, $18, $19, $20, $21, $22)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(body.store_id)
    .bind(&name)
    .bind(&body.kind)
    .bind(&body.discount)
    .bind(body.discount_value)
    .bind(body.coupon_code.as_deref().map(|c| c.trim().to_uppercase()))
    .bind(&body.product_ids)
    .bind(&body.category_ids)
    .bind(&body.bundle_product_ids)
    .bind(body.customer_group_id)
    .bind(body.first_order_only)
    .bind(body.min_spend_minor)
    .bind(body.min_quantity)
    .bind(body.flash_price_minor)
    .bind(body.flash_stock_cap)
    .bind(body.stackable)
    .bind(body.starts_at)
    .bind(body.ends_at)
    .bind(body.total_usage_limit)
    .bind(body.per_customer_limit)
    .execute(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db) if db.is_unique_violation() => ApiError::conflict_with_code("That coupon code is already in use.", "ALREADY_EXISTS"),
        other => ApiError::from(other),
    })?;
    let created = detail(&mut tx, id).await?;
    audit::record(&mut tx, tenant_id, user_id, "promotion", id, "promotion.create", serde_json::to_value(&created).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Promotion created.", created)));
}

/// `GET /api/v1/promotions/:id`. With how often it has been used.
pub async fn show_promotion(Path(promotion_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PromotionDetail>>, ApiError> {
    let found = detail(&mut tx, promotion_id).await?;
    assert_manage(&mut tx, found.promotion.store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Promotion fetched.", found)));
}

/// `PATCH /api/v1/promotions/:id`. What it discounts is fixed once created; its terms and window can change.
pub async fn update_promotion(
    Path(promotion_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdatePromotionRequest>,
) -> Result<Json<ApiEnvelope<PromotionDetail>>, ApiError> {
    let before = detail(&mut tx, promotion_id).await?;
    assert_manage(&mut tx, before.promotion.store_id).await?;
    if body.discount_value.is_some_and(|v| v < 0 || (before.promotion.discount == "percentage" && v > engine::BASIS_POINTS)) {
        return Err(ApiError::bad_request_with_code("discount_value is out of range", "VALIDATION_FAILED"));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    sqlx::query(
        "UPDATE atlas.promotions SET
             name = COALESCE($2, name), discount_value = COALESCE($3, discount_value), stackable = COALESCE($4, stackable),
             starts_at = COALESCE($5, starts_at), ends_at = COALESCE($6, ends_at),
             total_usage_limit = COALESCE($7, total_usage_limit), per_customer_limit = COALESCE($8, per_customer_limit),
             is_active = COALESCE($9, is_active)
         WHERE id = $1",
    )
    .bind(promotion_id)
    .bind(body.name.as_deref().map(str::trim))
    .bind(body.discount_value)
    .bind(body.stackable)
    .bind(body.starts_at)
    .bind(body.ends_at)
    .bind(body.total_usage_limit)
    .bind(body.per_customer_limit)
    .bind(body.is_active)
    .execute(&mut *tx)
    .await?;
    let after = detail(&mut tx, promotion_id).await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "promotion",
            entity_id: promotion_id,
            action: "promotion.update",
            before: serde_json::to_value(&before).ok(),
            after: serde_json::to_value(&after).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Promotion updated.", after)));
}
