use axum::extract::{Path, Query};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::roles::service::StoreScope;
use crate::sales::receipts;
use crate::sales::service as sales;

use super::models::{CreateReturnRequest, ListReturnsQuery, RejectReturnRequest, Return, ReturnDetail, ReturnsPage};
use super::{receipt, service};

/// Reading returns: `sale.view` or `sale.return` at the store.
async fn assert_can_read(tx: &mut TenantTx, store_id: Uuid) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if roles::service::has_permission(tx, user_id, tenant_id, Some(store_id), "sale.return").await? {
        return Ok(());
    }
    return roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), "sale.view").await;
}

/**
 * `POST /api/v1/sales/:id/returns`.
 * Requires `sale.return` at the sale's store. Completes straight away when the requester may authorise the refund and it's inside the window and under the approval threshold; otherwise it waits as `pending_approval`. See `service::create_return`.
 */
pub async fn create_return(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CreateReturnRequest>,
) -> Result<Json<ApiEnvelope<ReturnDetail>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let store_id = crate::payments::service::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.return").await?;
    let detail = service::create_return(&mut tx, tenant_id, sale_id, user_id, &body).await?;
    tx.commit().await?;
    let message = if detail.header.status == "completed" { "Return completed." } else { "Return recorded - waiting for approval." };
    return Ok(Json(ApiEnvelope::ok(message, detail)));
}

/**
 * `POST /api/v1/returns/:id/approve`.
 * Requires `sale.refund` at the store, and a different person from whoever started the return.
 */
pub async fn approve_return(Path(return_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ReturnDetail>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let header = service::get_return(&mut tx, return_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(header.store_id), "sale.refund").await?;
    let detail = service::approve(&mut tx, tenant_id, return_id, user_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "return", return_id, "return.approve", None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Return approved and completed.", detail)));
}

/// `POST /api/v1/returns/:id/reject`. Requires `sale.refund` at the store.
pub async fn reject_return(
    Path(return_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<RejectReturnRequest>,
) -> Result<Json<ApiEnvelope<ReturnDetail>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let header = service::get_return(&mut tx, return_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(header.store_id), "sale.refund").await?;
    let detail = service::reject(&mut tx, return_id, user_id, &body.reason).await?;
    audit::record(&mut tx, tenant_id, user_id, "return", return_id, "return.reject", Some(serde_json::json!({ "reason": body.reason }))).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Return rejected.", detail)));
}

/// `GET /api/v1/returns/:id`.
pub async fn get_return(Path(return_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ReturnDetail>>, ApiError> {
    let header = service::get_return(&mut tx, return_id).await?;
    assert_can_read(&mut tx, header.store_id).await?;
    let detail = service::detail(&mut tx, header).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Return fetched.", detail)));
}

/// `GET /api/v1/sales/:id/returns`.
pub async fn list_for_sale(Path(sale_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Return>>>, ApiError> {
    let store_id = crate::payments::service::sale_store_id(&mut tx, sale_id).await?;
    assert_can_read(&mut tx, store_id).await?;
    let returns = sqlx::query_scalar::<_, Uuid>("SELECT id FROM atlas.returns WHERE sale_id = $1 ORDER BY created_at")
        .bind(sale_id)
        .fetch_all(&mut *tx)
        .await?;
    let mut headers = Vec::with_capacity(returns.len());
    for id in returns {
        headers.push(service::get_return(&mut tx, id).await?);
    }
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Returns fetched.", headers)));
}

/**
 * `GET /api/v1/returns?store_id=&status=&from=&to=&cursor=&limit=`.
 * Covers every store where the caller holds `sale.view` or `sale.return`.
 */
pub async fn list_returns(mut tx: TenantTx, Query(query): Query<ListReturnsQuery>) -> Result<Json<ApiEnvelope<ReturnsPage>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let view = roles::service::stores_with_permission(&mut tx, user_id, tenant_id, "sale.view").await?;
    let ret = roles::service::stores_with_permission(&mut tx, user_id, tenant_id, "sale.return").await?;
    let scope = match (view, ret) {
        (StoreScope::All, _) | (_, StoreScope::All) => StoreScope::All,
        (StoreScope::Stores(mut a), StoreScope::Stores(b)) => {
            a.extend(b);
            StoreScope::Stores(a)
        }
    };
    if scope == StoreScope::Stores(vec![]) || query.store_id.is_some_and(|s| !scope.includes(s)) {
        return Err(ApiError::forbidden("Missing permission: sale.view"));
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM atlas.returns
         WHERE ($1::uuid[] IS NULL OR store_id = ANY($1))
           AND ($2::uuid IS NULL OR store_id = $2)
           AND ($3::text IS NULL OR status::text = $3)
           AND ($4::timestamptz IS NULL OR created_at >= $4)
           AND ($5::timestamptz IS NULL OR created_at < $5)
           AND ($6::uuid IS NULL OR (created_at, id) < (SELECT created_at, id FROM atlas.returns WHERE id = $6))
         ORDER BY created_at DESC, id DESC
         LIMIT $7",
    )
    .bind(scope.as_filter())
    .bind(query.store_id)
    .bind(&query.status)
    .bind(query.from)
    .bind(query.to)
    .bind(query.cursor)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;
    let mut items = Vec::with_capacity(ids.len());
    for id in &ids {
        items.push(service::get_return(&mut tx, *id).await?);
    }
    tx.commit().await?;
    let next_cursor = if ids.len() as i64 == limit { ids.last().copied() } else { None };
    return Ok(Json(ApiEnvelope::ok("Returns fetched.", ReturnsPage { items, next_cursor })));
}

#[derive(Deserialize)]
pub struct ReturnReceiptQuery {
    pub width: Option<usize>,
}

/// `GET /api/v1/returns/:id/receipt?width=` - the plain-text return slip.
pub async fn return_receipt(
    Path(return_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(params): Query<ReturnReceiptQuery>,
) -> Result<Response, ApiError> {
    let header = service::get_return(&mut tx, return_id).await?;
    assert_can_read(&mut tx, header.store_id).await?;
    sales::get_sale(&mut tx, header.sale_id).await?;
    let sale_receipt = receipts::view(&mut tx, header.sale_id).await?;
    let detail = service::detail(&mut tx, header).await?;
    tx.commit().await?;
    let text = receipt::render_text(&sale_receipt.snapshot, &detail, params.width.unwrap_or(32));
    return Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response());
}
