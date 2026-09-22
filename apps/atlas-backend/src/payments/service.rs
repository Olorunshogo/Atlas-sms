use chrono::{Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::ApiError;

use super::models::{CreatePaymentRequest, Payment, Refund};

/// How long a refund's confirmation token is valid before it must be re-requested - long enough for a cashier to double-check with a manager, short enough that a token left on screen doesn't stay a live "execute this refund" button indefinitely.
const CONFIRMATION_WINDOW_MINUTES: i64 = 15;

pub async fn sale_store_id(conn: &mut PgConnection, sale_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.sales WHERE id = $1")
        .bind(sale_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::bad_request("Unknown sale."));
}

/// The store a payment's own permission check is scoped to - one join away from `sale_store_id`, since a payment only carries `sale_id`, not `store_id` directly.
pub async fn payment_store_id(conn: &mut PgConnection, payment_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar(
        "SELECT s.store_id FROM atlas.payments p JOIN atlas.sales s ON s.id = p.sale_id WHERE p.id = $1",
    )
    .bind(payment_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown payment."));
}

/**
 * Same idea, one join further - the store a refund's own permission check is scoped to.
 * A store-scoped `sale.refund` grant must be checked against this, not `None`/tenant-wide - `assert_permission`'s SQL never matches a store-scope grant when `store_id` is `None`, so passing `None` here would silently exclude every store_manager from confirming a refund at their own store.
 */
pub async fn refund_store_id(conn: &mut PgConnection, refund_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar(
        "SELECT s.store_id FROM atlas.refunds r
         JOIN atlas.payments p ON p.id = r.payment_id
         JOIN atlas.sales s ON s.id = p.sale_id
         WHERE r.id = $1",
    )
    .bind(refund_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Unknown refund."));
}

pub async fn create_payment(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreatePaymentRequest,
) -> Result<Payment, ApiError> {
    // Recorded as 'succeeded' immediately - v1 has no payment gateway to
    // wait on a callback from, only in-person methods a cashier already
    // confirmed by the time this call happens. 'pending'/'failed' stay in
    // the enum for whenever a real gateway integration needs them.
    return sqlx::query_as::<_, Payment>(
        "INSERT INTO atlas.payments (id, tenant_id, sale_id, method, amount_minor, currency, status)
         VALUES ($1, $2, $3, $4::atlas.payment_method, $5, $6, 'succeeded')
         RETURNING id, sale_id, method::text AS method, amount_minor, currency, status::text AS status",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.sale_id)
    .bind(&payload.method)
    .bind(payload.amount_minor)
    .bind(&payload.currency)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn list_payments_for_sale(
    conn: &mut PgConnection,
    sale_id: Uuid,
) -> Result<Vec<Payment>, ApiError> {
    return sqlx::query_as::<_, Payment>(
        "SELECT id, sale_id, method::text AS method, amount_minor, currency, status::text AS status
         FROM atlas.payments WHERE sale_id = $1 ORDER BY created_at",
    )
    .bind(sale_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * First half of the two-step refund - mints and stores a hashed confirmation token, returns the raw one to the caller exactly once.
 * Executes nothing yet; see `confirm_refund`.
 */
pub async fn request_refund(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    requested_by_user_id: Uuid,
) -> Result<(Refund, String), ApiError> {
    let token = crypto::generate_opaque_token();
    let token_hash = crypto::hash_lookup_token(&token);
    let expires_at = Utc::now() + Duration::minutes(CONFIRMATION_WINDOW_MINUTES);

    let refund = sqlx::query_as::<_, Refund>(
        "INSERT INTO atlas.refunds
            (id, tenant_id, payment_id, amount_minor, confirmation_token_hash, confirmation_expires_at, requested_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, payment_id, amount_minor, status::text AS status",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payment_id)
    .bind(amount_minor)
    .bind(&token_hash)
    .bind(expires_at)
    .bind(requested_by_user_id)
    .fetch_one(conn)
    .await?;

    return Ok((refund, token));
}

/**
 * Second half - executes a refund that's still `awaiting_confirmation`, its token matches, and the confirmation window hasn't expired.
 * Any other state (already executed, wrong token, expired) is a flat rejection, not a partial success.
 */
pub async fn confirm_refund(
    conn: &mut PgConnection,
    refund_id: Uuid,
    confirmation_token: &str,
) -> Result<Refund, ApiError> {
    let token_hash = crypto::hash_lookup_token(confirmation_token);

    return sqlx::query_as::<_, Refund>(
        "UPDATE atlas.refunds
         SET status = 'executed'
         WHERE id = $1
           AND status = 'awaiting_confirmation'
           AND confirmation_token_hash = $2
           AND confirmation_expires_at > now()
         RETURNING id, payment_id, amount_minor, status::text AS status",
    )
    .bind(refund_id)
    .bind(&token_hash)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Invalid, expired, or already-resolved refund confirmation."));
}
