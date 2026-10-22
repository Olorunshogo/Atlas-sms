/*!
 * Sales history (FR-17): search by date, store, cashier, customer,
 * transaction id, receipt number and payment status, plus the access rule
 * every read of a single sale goes through.
 */
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::roles;
use crate::roles::service::StoreScope;

use super::models::Sale;

#[derive(Debug, Deserialize)]
pub struct SalesQuery {
    pub store_id: Option<Uuid>,
    pub shift_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cashier_user_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub status: Option<String>,
    pub payment_status: Option<String>,
    /// An exact receipt number, or the start of a transaction id.
    pub q: Option<String>,
    /// The id of the last sale on the previous page.
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleSummary {
    pub id: Uuid,
    pub receipt_number: Option<String>,
    pub store_id: Uuid,
    pub store_name: String,
    pub shift_id: Uuid,
    pub cashier_user_id: Uuid,
    pub cashier_name: String,
    pub customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
    pub status: String,
    pub payment_status: String,
    pub currency: String,
    pub total_amount_minor: i64,
    pub amount_paid_minor: i64,
    pub amount_refunded_minor: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SalesPage {
    pub items: Vec<SaleSummary>,
    pub next_cursor: Option<Uuid>,
}

/**
 * Callers with `sale.view` search every store it reaches. Everyone else - a cashier - sees only their own sales in shifts that are still open: enough to find the sale they just rang up, not a window into the shop's history.
 */
pub async fn search(tx: &mut TenantTx, query: &SalesQuery) -> Result<SalesPage, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let scope = roles::service::stores_with_permission(tx, user_id, tenant_id, "sale.view").await?;
    let restricted = scope == StoreScope::Stores(vec![]);
    if let (Some(store_id), false) = (query.store_id, restricted) {
        if !scope.includes(store_id) {
            return Err(ApiError::forbidden("Missing permission: sale.view"));
        }
    }
    let cashier = if restricted { Some(user_id) } else { query.cashier_user_id };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let q = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());

    let items = sqlx::query_as::<_, SaleSummary>(
        "SELECT s.id, s.receipt_number, s.store_id, st.name AS store_name, s.shift_id, s.cashier_user_id,
                u.full_name AS cashier_name, s.customer_id, c.full_name AS customer_name, s.status::text AS status,
                s.payment_status::text AS payment_status, s.currency, s.total_amount_minor, s.amount_paid_minor,
                s.amount_refunded_minor, s.created_at, s.completed_at
         FROM atlas.sales s
         JOIN atlas.stores st ON st.id = s.store_id
         JOIN atlas.users u ON u.id = s.cashier_user_id
         JOIN atlas.shifts sh ON sh.id = s.shift_id
         LEFT JOIN atlas.customers c ON c.id = s.customer_id
         WHERE ($1::uuid[] IS NULL OR s.store_id = ANY($1))
           AND ($2::uuid IS NULL OR s.store_id = $2)
           AND ($3::uuid IS NULL OR s.shift_id = $3)
           AND ($4::timestamptz IS NULL OR s.created_at >= $4)
           AND ($5::timestamptz IS NULL OR s.created_at < $5)
           AND ($6::uuid IS NULL OR s.cashier_user_id = $6)
           AND ($7::uuid IS NULL OR s.customer_id = $7)
           AND ($8::text IS NULL OR s.status::text = $8)
           AND ($9::text IS NULL OR s.payment_status::text = $9)
           AND ($10::text IS NULL OR s.receipt_number = $10 OR s.id::text LIKE lower($10) || '%')
           AND ($11::uuid IS NULL OR (s.created_at, s.id) < (SELECT created_at, id FROM atlas.sales WHERE id = $11))
           AND (NOT $12 OR sh.status = 'open')
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT $13",
    )
    .bind(if restricted { None } else { scope.as_filter() })
    .bind(query.store_id)
    .bind(query.shift_id)
    .bind(query.from)
    .bind(query.to)
    .bind(cashier)
    .bind(query.customer_id)
    .bind(&query.status)
    .bind(&query.payment_status)
    .bind(q)
    .bind(query.cursor)
    .bind(restricted)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    let next_cursor = if items.len() as i64 == limit { items.last().map(|s| s.id) } else { None };
    return Ok(SalesPage { items, next_cursor });
}

/// May this caller read this sale? `sale.view` at its store, or it's their own sale in a shift that's still open.
pub async fn can_view(conn: &mut PgConnection, user_id: Uuid, tenant_id: Uuid, sale: &Sale) -> Result<bool, ApiError> {
    if roles::service::has_permission(conn, user_id, tenant_id, Some(sale.store_id), "sale.view").await? {
        return Ok(true);
    }
    if sale.cashier_user_id != user_id {
        return Ok(false);
    }
    let open: bool = sqlx::query_scalar("SELECT status = 'open' FROM atlas.shifts WHERE id = $1")
        .bind(sale.shift_id)
        .fetch_one(conn)
        .await?;
    return Ok(open);
}

pub async fn assert_can_view(conn: &mut PgConnection, user_id: Uuid, tenant_id: Uuid, sale: &Sale) -> Result<(), ApiError> {
    if !can_view(conn, user_id, tenant_id, sale).await? {
        return Err(ApiError::forbidden("Missing permission: sale.view"));
    }
    return Ok(());
}

pub async fn find_by_receipt(conn: &mut PgConnection, receipt_number: &str) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT id FROM atlas.sales WHERE receipt_number = $1")
        .bind(receipt_number.trim())
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("No sale has that receipt number."));
}
