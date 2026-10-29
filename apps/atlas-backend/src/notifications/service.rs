use std::collections::BTreeSet;

use rust_decimal::Decimal;
use sqlx::{Connection, PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{Audience, Event, ListNotificationsQuery, Notification, NotificationKind};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

const NOTIFICATION_COLUMNS: &str = "n.id, n.store_id, n.kind, n.title, n.body, n.entity_type, n.entity_id, n.created_at, r.status, r.read_at";

// === Isolation

/**
 * Every write below runs in its own savepoint, so a failed notification is
 * logged and dropped instead of rolling back the sale or transfer that
 * raised it (FR-26).
 */
async fn begin<'c>(conn: &'c mut PgConnection, what: &str) -> Option<Transaction<'c, Postgres>> {
    return match conn.begin().await {
        Ok(savepoint) => Some(savepoint),
        Err(err) => {
            eprintln!("notification {} skipped: {}", what, err);
            None
        }
    };
}

/// Returns what went wrong, if anything, so the caller can leave an operational issue behind (FR-30).
async fn finish(savepoint: Transaction<'_, Postgres>, result: Result<(), ApiError>, what: &str) -> Option<String> {
    match result {
        Ok(()) => match savepoint.commit().await {
            Ok(()) => None,
            Err(err) => {
                eprintln!("notification {} not saved: {}", what, err);
                Some(format!("{} notification not saved: {}", what, err))
            }
        },
        Err(err) => {
            eprintln!("notification {} dropped: {:?}", what, err);
            let _ = savepoint.rollback().await;
            Some(format!("{} notification dropped: {:?}", what, err))
        }
    }
}

/// Best effort, in its own savepoint: failing to record the failure must not fail the request either.
pub async fn record_issue(conn: &mut PgConnection, tenant_id: Uuid, kind: &str, detail: &str, reference_id: Option<Uuid>) {
    let Ok(mut savepoint) = conn.begin().await else { return };
    let inserted = sqlx::query(
        "INSERT INTO atlas.operational_issues (id, tenant_id, kind, detail, reference_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(kind)
    .bind(detail)
    .bind(reference_id)
    .execute(&mut *savepoint)
    .await;
    match inserted {
        Ok(_) => {
            let _ = savepoint.commit().await;
        }
        Err(err) => {
            eprintln!("operational issue not recorded: {}", err);
            let _ = savepoint.rollback().await;
        }
    }
}

/// For a failure that also fails the request: its transaction rolls back, so the issue goes on a connection of its own (FR-30).
pub async fn record_issue_detached(pool: &sqlx::PgPool, tenant_id: Uuid, kind: &str, detail: &str, reference_id: Option<Uuid>) {
    let Ok(mut tx) = pool.begin().await else { return };
    let scoped = sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await;
    if scoped.is_err() {
        return;
    }
    record_issue(&mut tx, tenant_id, kind, detail, reference_id).await;
    if let Err(err) = tx.commit().await {
        eprintln!("operational issue not recorded: {}", err);
    }
}

// === Writing

async fn recipients(conn: &mut PgConnection, event: &Event) -> Result<BTreeSet<Uuid>, ApiError> {
    let mut users = BTreeSet::new();
    for audience in &event.audience {
        match audience {
            Audience::User(user_id) => {
                users.insert(*user_id);
            }
            Audience::Permission { permission, store_id } => {
                let holders: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT rg.user_id
                     FROM atlas.role_grants rg
                     JOIN atlas.roles r ON r.id = rg.role_id
                     WHERE rg.status = 'active'
                       AND $1::atlas.permission = ANY(r.permissions)
                       AND (
                             (rg.scope = 'tenant' AND rg.tenant_id = $2)
                          OR (rg.scope = 'store' AND rg.store_id = $3)
                          OR EXISTS (
                               SELECT 1 FROM atlas.store_group_grants sgg
                               WHERE sgg.role_grant_id = rg.id AND sgg.store_id = $3
                             )
                       )",
                )
                .bind(permission)
                .bind(event.tenant_id)
                .bind(store_id)
                .fetch_all(&mut *conn)
                .await?;
                users.extend(holders);
            }
            Audience::TenantPermission { permission } => {
                let holders: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT rg.user_id
                     FROM atlas.role_grants rg
                     JOIN atlas.roles r ON r.id = rg.role_id
                     WHERE rg.status = 'active' AND rg.scope = 'tenant' AND rg.tenant_id = $2
                       AND $1::atlas.permission = ANY(r.permissions)",
                )
                .bind(permission)
                .bind(event.tenant_id)
                .fetch_all(&mut *conn)
                .await?;
                users.extend(holders);
            }
        }
    }
    if let Some(excluded) = event.exclude_user_id {
        users.remove(&excluded);
    }
    return Ok(users);
}

async fn write_event(conn: &mut PgConnection, event: &Event) -> Result<(), ApiError> {
    let users = recipients(conn, event).await?;
    if users.is_empty() {
        return Ok(());
    }

    // An open alert with the same key already covers this condition
    let notification_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO atlas.notifications
            (id, tenant_id, store_id, kind, title, body, entity_type, entity_id, dedupe_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (tenant_id, dedupe_key) WHERE dedupe_key IS NOT NULL AND resolved_at IS NULL DO NOTHING
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(event.tenant_id)
    .bind(event.store_id)
    .bind(event.kind)
    .bind(&event.title)
    .bind(&event.body)
    .bind(event.entity_type)
    .bind(event.entity_id)
    .bind(&event.dedupe_key)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(notification_id) = notification_id else { return Ok(()) };

    // No preference row means in-app on, email off
    let users: Vec<Uuid> = users.into_iter().collect();
    sqlx::query(
        "INSERT INTO atlas.notification_recipients (notification_id, user_id, tenant_id)
         SELECT $1, u.user_id, $2 FROM UNNEST($3::uuid[]) AS u(user_id)
         LEFT JOIN atlas.notification_preferences p ON p.user_id = u.user_id AND p.tenant_id = $2 AND p.kind = $4
         WHERE COALESCE(p.in_app, true)",
    )
    .bind(notification_id)
    .bind(event.tenant_id)
    .bind(&users)
    .bind(event.kind)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO atlas.notification_deliveries (id, tenant_id, notification_id, user_id)
         SELECT gen_random_uuid(), $2, $1, p.user_id FROM atlas.notification_preferences p
         WHERE p.user_id = ANY($3) AND p.tenant_id = $2 AND p.kind = $4 AND p.email",
    )
    .bind(notification_id)
    .bind(event.tenant_id)
    .bind(&users)
    .bind(event.kind)
    .execute(&mut *conn)
    .await?;
    return Ok(());
}

// === Low stock (FR-13)

/**
 * Called by `inventory::service::post` with each product's net change.
 * A decrease that leaves stock at or below the listing's threshold opens
 * an alert; any change that lifts it back above resolves the open one.
 */
pub async fn check_low_stock(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, changes: &[(Uuid, Decimal)]) {
    let Some(mut savepoint) = begin(conn, "low stock").await else { return };
    let result = write_low_stock(&mut savepoint, tenant_id, store_id, changes).await;
    if let Some(error) = finish(savepoint, result, "low stock").await {
        record_issue(conn, tenant_id, "notification_failed", &error, None).await;
    }
}

async fn write_low_stock(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, changes: &[(Uuid, Decimal)]) -> Result<(), ApiError> {
    for (product_id, net) in changes {
        let listing: Option<(Decimal, Decimal, String, String)> = sqlx::query_as(
            "SELECT l.low_stock_threshold,
                    (SELECT COALESCE(SUM(m.quantity), 0) FROM atlas.stock_movements m
                     WHERE m.store_id = l.store_id AND m.product_id = l.product_id),
                    p.name, s.name
             FROM atlas.product_store_listings l
             JOIN atlas.products p ON p.id = l.product_id
             JOIN atlas.stores s ON s.id = l.store_id
             WHERE l.product_id = $1 AND l.store_id = $2",
        )
        .bind(product_id)
        .bind(store_id)
        .fetch_optional(&mut *conn)
        .await?;
        let Some((threshold, on_hand, product_name, store_name)) = listing else { continue };
        let dedupe_key = format!("low_stock:{}:{}", store_id, product_id);

        if on_hand > threshold {
            sqlx::query(
                "UPDATE atlas.notifications SET resolved_at = now()
                 WHERE tenant_id = $1 AND dedupe_key = $2 AND resolved_at IS NULL",
            )
            .bind(tenant_id)
            .bind(&dedupe_key)
            .execute(&mut *conn)
            .await?;
        } else if *net < Decimal::ZERO {
            let event = Event {
                tenant_id,
                store_id: Some(store_id),
                kind: NotificationKind::LowStock,
                title: format!("Low stock: {}", product_name),
                body: format!(
                    "{} has {} left at {}, at or below its reorder level of {}.",
                    product_name,
                    on_hand.normalize(),
                    store_name,
                    threshold.normalize()
                ),
                entity_type: "product",
                entity_id: *product_id,
                dedupe_key: Some(dedupe_key),
                audience: vec![
                    Audience::Permission { permission: "stock.adjust", store_id },
                    Audience::Permission { permission: "purchase.create", store_id },
                ],
                exclude_user_id: None,
            };
            write_event(conn, &event).await?;
        }
    }
    return Ok(());
}

// === Transfers (FR-22)

pub async fn transfer(conn: &mut PgConnection, tenant_id: Uuid, actor_user_id: Uuid, transfer_id: Uuid, kind: NotificationKind) {
    let Some(mut savepoint) = begin(conn, "transfer").await else { return };
    let result = write_transfer(&mut savepoint, tenant_id, actor_user_id, transfer_id, kind).await;
    if let Some(error) = finish(savepoint, result, "transfer").await {
        record_issue(conn, tenant_id, "notification_failed", &error, Some(transfer_id)).await;
    }
}

async fn write_transfer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    transfer_id: Uuid,
    kind: NotificationKind,
) -> Result<(), ApiError> {
    let (from_store_id, to_store_id, requested_by, quantity, product_name, from_name, to_name): (Uuid, Uuid, Uuid, Decimal, String, String, String) =
        sqlx::query_as(
            "SELECT t.from_store_id, t.to_store_id, t.requested_by_user_id, t.quantity, p.name, fs.name, ts.name
             FROM atlas.stock_transfers t
             JOIN atlas.products p ON p.id = t.product_id
             JOIN atlas.stores fs ON fs.id = t.from_store_id
             JOIN atlas.stores ts ON ts.id = t.to_store_id
             WHERE t.id = $1",
        )
        .bind(transfer_id)
        .fetch_one(&mut *conn)
        .await?;

    let to_receiving = Audience::Permission { permission: "stock.transfer", store_id: to_store_id };
    let to_sending = Audience::Permission { permission: "stock.transfer", store_id: from_store_id };
    let (title, store_id, audience) = match kind {
        NotificationKind::TransferRequested => ("Transfer requested", to_store_id, vec![to_receiving]),
        NotificationKind::TransferApproved => ("Transfer approved", from_store_id, vec![Audience::User(requested_by), to_sending]),
        NotificationKind::TransferRejected => ("Transfer rejected", from_store_id, vec![Audience::User(requested_by), to_sending]),
        NotificationKind::TransferDispatched => ("Transfer dispatched", to_store_id, vec![to_receiving]),
        NotificationKind::TransferReceived => ("Transfer received", from_store_id, vec![Audience::User(requested_by), to_sending]),
        _ => return Ok(()),
    };

    let event = Event {
        tenant_id,
        store_id: Some(store_id),
        kind,
        title: title.to_string(),
        body: format!("{} of {} from {} to {}.", quantity.normalize(), product_name, from_name, to_name),
        entity_type: "stock_transfer",
        entity_id: transfer_id,
        dedupe_key: None,
        audience,
        exclude_user_id: Some(actor_user_id),
    };
    return write_event(conn, &event).await;
}

// === Stock counts (FR-12)

pub async fn stock_count(conn: &mut PgConnection, tenant_id: Uuid, actor_user_id: Uuid, count_id: Uuid, kind: NotificationKind) {
    let Some(mut savepoint) = begin(conn, "stock count").await else { return };
    let result = write_stock_count(&mut savepoint, tenant_id, actor_user_id, count_id, kind).await;
    if let Some(error) = finish(savepoint, result, "stock count").await {
        record_issue(conn, tenant_id, "notification_failed", &error, Some(count_id)).await;
    }
}

async fn write_stock_count(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    count_id: Uuid,
    kind: NotificationKind,
) -> Result<(), ApiError> {
    let (store_id, store_name, lines, off): (Uuid, String, i64, i64) = sqlx::query_as(
        "SELECT c.store_id, s.name,
                (SELECT COUNT(*) FROM atlas.stock_count_lines l WHERE l.stock_count_id = c.id),
                (SELECT COUNT(*) FROM atlas.stock_count_lines l WHERE l.stock_count_id = c.id AND l.variance <> 0)
         FROM atlas.stock_counts c
         JOIN atlas.stores s ON s.id = c.store_id
         WHERE c.id = $1",
    )
    .bind(count_id)
    .fetch_one(&mut *conn)
    .await?;

    let event = Event {
        tenant_id,
        store_id: Some(store_id),
        kind,
        title: "Stock count awaiting approval".to_string(),
        body: format!("{} lines counted at {}, {} with a variance.", lines, store_name, off),
        entity_type: "stock_count",
        entity_id: count_id,
        dedupe_key: None,
        audience: vec![Audience::Permission { permission: "stock.adjust", store_id }],
        exclude_user_id: Some(actor_user_id),
    };
    return write_event(conn, &event).await;
}

// === Purchase orders (FR-21)

pub async fn purchase_order(conn: &mut PgConnection, tenant_id: Uuid, actor_user_id: Uuid, purchase_order_id: Uuid, kind: NotificationKind) {
    let Some(mut savepoint) = begin(conn, "purchase order").await else { return };
    let result = write_purchase_order(&mut savepoint, tenant_id, actor_user_id, purchase_order_id, kind).await;
    if let Some(error) = finish(savepoint, result, "purchase order").await {
        record_issue(conn, tenant_id, "notification_failed", &error, Some(purchase_order_id)).await;
    }
}

async fn write_purchase_order(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    purchase_order_id: Uuid,
    kind: NotificationKind,
) -> Result<(), ApiError> {
    let (store_id, created_by, supplier_name, store_name): (Uuid, Uuid, String, String) = sqlx::query_as(
        "SELECT po.store_id, po.created_by_user_id, sup.name, s.name
         FROM atlas.purchase_orders po
         JOIN atlas.suppliers sup ON sup.id = po.supplier_id
         JOIN atlas.stores s ON s.id = po.store_id
         WHERE po.id = $1",
    )
    .bind(purchase_order_id)
    .fetch_one(&mut *conn)
    .await?;

    let (title, body, audience) = match kind {
        NotificationKind::PurchaseOrderSubmitted => (
            "Purchase order awaiting approval",
            format!("Order to {} for {} needs approving.", supplier_name, store_name),
            vec![Audience::Permission { permission: "purchase.approve", store_id }],
        ),
        NotificationKind::PurchaseOrderApproved => (
            "Purchase order approved",
            format!("Order to {} for {} is approved and can be received.", supplier_name, store_name),
            vec![Audience::User(created_by)],
        ),
        _ => return Ok(()),
    };

    let event = Event {
        tenant_id,
        store_id: Some(store_id),
        kind,
        title: title.to_string(),
        body,
        entity_type: "purchase_order",
        entity_id: purchase_order_id,
        dedupe_key: None,
        audience,
        exclude_user_id: Some(actor_user_id),
    };
    return write_event(conn, &event).await;
}

// === Expenses (FR-24)

pub async fn expense(conn: &mut PgConnection, tenant_id: Uuid, actor_user_id: Uuid, expense_id: Uuid, kind: NotificationKind) {
    let Some(mut savepoint) = begin(conn, "expense").await else { return };
    let result = write_expense(&mut savepoint, tenant_id, actor_user_id, expense_id, kind).await;
    if let Some(error) = finish(savepoint, result, "expense").await {
        record_issue(conn, tenant_id, "notification_failed", &error, Some(expense_id)).await;
    }
}

async fn write_expense(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    expense_id: Uuid,
    kind: NotificationKind,
) -> Result<(), ApiError> {
    let (store_id, created_by, amount_minor, description, store_name): (Uuid, Uuid, i64, String, String) = sqlx::query_as(
        "SELECT e.store_id, e.created_by_user_id, e.amount_minor, e.description, s.name
         FROM atlas.expenses e
         JOIN atlas.stores s ON s.id = e.store_id
         WHERE e.id = $1",
    )
    .bind(expense_id)
    .fetch_one(&mut *conn)
    .await?;

    let (title, audience) = match kind {
        NotificationKind::ExpenseSubmitted => ("Expense awaiting approval", vec![Audience::Permission { permission: "expense.manage", store_id }]),
        NotificationKind::ExpenseApproved => ("Expense approved", vec![Audience::User(created_by)]),
        NotificationKind::ExpenseRejected => ("Expense rejected", vec![Audience::User(created_by)]),
        _ => return Ok(()),
    };

    let event = Event {
        tenant_id,
        store_id: Some(store_id),
        kind,
        title: title.to_string(),
        body: format!("{} ({}.{:02}) at {}.", description, amount_minor / 100, amount_minor % 100, store_name),
        entity_type: "expense",
        entity_id: expense_id,
        dedupe_key: None,
        audience,
        exclude_user_id: Some(actor_user_id),
    };
    return write_event(conn, &event).await;
}

// === Approval requests (FR-09, FR-29)

pub async fn approval(conn: &mut PgConnection, tenant_id: Uuid, actor_user_id: Uuid, request_id: Uuid, kind: NotificationKind) {
    let Some(mut savepoint) = begin(conn, "approval").await else { return };
    let result = write_approval(&mut savepoint, tenant_id, actor_user_id, request_id, kind).await;
    if let Some(error) = finish(savepoint, result, "approval").await {
        record_issue(conn, tenant_id, "notification_failed", &error, Some(request_id)).await;
    }
}

async fn write_approval(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    request_id: Uuid,
    kind: NotificationKind,
) -> Result<(), ApiError> {
    let (store_id, operation, status, summary, requested_by): (Option<Uuid>, String, String, String, Uuid) = sqlx::query_as(
        "SELECT store_id, operation::text, status::text, summary, requested_by_user_id FROM atlas.approval_requests WHERE id = $1",
    )
    .bind(request_id)
    .fetch_one(&mut *conn)
    .await?;
    let permission = if operation == "price_change" { "price.approve" } else { "stock.approve" };

    let (title, audience) = match kind {
        NotificationKind::ApprovalRequested => (
            "Approval needed".to_string(),
            vec![match store_id {
                Some(store_id) => Audience::Permission { permission, store_id },
                None => Audience::TenantPermission { permission },
            }],
        ),
        NotificationKind::ApprovalDecided => (format!("Request {}", status), vec![Audience::User(requested_by)]),
        _ => return Ok(()),
    };

    let event = Event {
        tenant_id,
        store_id,
        kind,
        title,
        body: summary,
        entity_type: "approval_request",
        entity_id: request_id,
        dedupe_key: None,
        audience,
        exclude_user_id: Some(actor_user_id),
    };
    return write_event(conn, &event).await;
}

// === Inbox

pub async fn list(conn: &mut PgConnection, user_id: Uuid, query: &ListNotificationsQuery) -> Result<Vec<Notification>, ApiError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    return sqlx::query_as::<_, Notification>(&format!(
        "SELECT {NOTIFICATION_COLUMNS}
         FROM atlas.notification_recipients r
         JOIN atlas.notifications n ON n.id = r.notification_id
         WHERE r.user_id = $1 AND ($2::atlas.notification_status IS NULL OR r.status = $2)
         ORDER BY n.created_at DESC
         LIMIT $3"
    ))
    .bind(user_id)
    .bind(query.status)
    .bind(limit)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn unread_count(conn: &mut PgConnection, user_id: Uuid) -> Result<i64, ApiError> {
    return sqlx::query_scalar("SELECT COUNT(*) FROM atlas.notification_recipients WHERE user_id = $1 AND status = 'unread'")
        .bind(user_id)
        .fetch_one(conn)
        .await
        .map_err(ApiError::from);
}

pub async fn mark_read(conn: &mut PgConnection, user_id: Uuid, notification_id: Uuid) -> Result<Notification, ApiError> {
    return sqlx::query_as::<_, Notification>(&format!(
        "WITH updated AS (
             UPDATE atlas.notification_recipients
             SET status = 'read', read_at = COALESCE(read_at, now())
             WHERE notification_id = $1 AND user_id = $2
             RETURNING notification_id, status, read_at
         )
         SELECT {NOTIFICATION_COLUMNS}
         FROM updated r
         JOIN atlas.notifications n ON n.id = r.notification_id"
    ))
    .bind(notification_id)
    .bind(user_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification not found."));
}

pub async fn mark_all_read(conn: &mut PgConnection, user_id: Uuid) -> Result<u64, ApiError> {
    let result = sqlx::query(
        "UPDATE atlas.notification_recipients SET status = 'read', read_at = now()
         WHERE user_id = $1 AND status = 'unread'",
    )
    .bind(user_id)
    .execute(conn)
    .await?;
    return Ok(result.rows_affected());
}
