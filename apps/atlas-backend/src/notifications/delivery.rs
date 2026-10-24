/*!
 * FR-26 optional email delivery, as an outbox. Raising a notification only queues a row; this worker sends it,
 * retrying with backoff (NFR-11), so a slow or failing mail provider never holds up the action behind it.
 */
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::email::EmailProvider;
use crate::error::ApiError;

pub const MAX_ATTEMPTS: i32 = 5;
const BATCH_SIZE: i64 = 50;

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct DeliveryRun {
    pub sent: u32,
    pub retrying: u32,
    pub failed: u32,
}

/**
 * Sends every email that is due. Rows are claimed with `SKIP LOCKED`, so two workers never send the same one.
 * A failure waits 2^attempts minutes; after `MAX_ATTEMPTS` the delivery is `failed` and an operational issue is raised.
 */
pub async fn deliver_pending(pool: &PgPool, email: &dyn EmailProvider) -> Result<DeliveryRun, ApiError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.bypass_tenant', 'on', true)")
        .execute(&mut *tx)
        .await?;

    let due: Vec<(Uuid, Uuid, Uuid, i32, String, String, String)> = sqlx::query_as(
        "SELECT d.id, d.tenant_id, d.notification_id, d.attempts, u.email, n.title, n.body
         FROM atlas.notification_deliveries d
         JOIN atlas.users u ON u.id = d.user_id
         JOIN atlas.notifications n ON n.id = d.notification_id
         WHERE d.status = 'pending' AND d.next_attempt_at <= now()
         ORDER BY d.next_attempt_at
         LIMIT $1
         FOR UPDATE OF d SKIP LOCKED",
    )
    .bind(BATCH_SIZE)
    .fetch_all(&mut *tx)
    .await?;

    let mut run = DeliveryRun::default();
    for (id, tenant_id, notification_id, attempts, to, subject, body) in due {
        let attempt = attempts + 1;
        match email.send(&to, &subject, &body).await {
            Ok(()) => {
                sqlx::query("UPDATE atlas.notification_deliveries SET status = 'sent', attempts = $2, sent_at = now(), last_error = NULL WHERE id = $1")
                    .bind(id)
                    .bind(attempt)
                    .execute(&mut *tx)
                    .await?;
                run.sent += 1;
            }
            Err(err) if attempt >= MAX_ATTEMPTS => {
                sqlx::query("UPDATE atlas.notification_deliveries SET status = 'failed', attempts = $2, last_error = $3 WHERE id = $1")
                    .bind(id)
                    .bind(attempt)
                    .bind(&err.0)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "INSERT INTO atlas.operational_issues (id, tenant_id, kind, detail, reference_id)
                     VALUES ($1, $2, 'email_delivery_failed', $3, $4)",
                )
                .bind(Uuid::now_v7())
                .bind(tenant_id)
                .bind(format!("Email to {} failed {} times: {}", to, attempt, err.0))
                .bind(notification_id)
                .execute(&mut *tx)
                .await?;
                run.failed += 1;
            }
            Err(err) => {
                sqlx::query(
                    "UPDATE atlas.notification_deliveries
                     SET attempts = $2, last_error = $3, next_attempt_at = now() + make_interval(mins => power(2, $2)::int)
                     WHERE id = $1",
                )
                .bind(id)
                .bind(attempt)
                .bind(&err.0)
                .execute(&mut *tx)
                .await?;
                run.retrying += 1;
            }
        }
    }
    tx.commit().await?;
    return Ok(run);
}
