use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{MethodTotals, Reconciliation, ReconciliationQuery, SalesTotals, ShiftCashSummary};

const SALE_FILTER: &str = "s.store_id = $1
       AND ($2::uuid IS NULL OR s.shift_id = $2)
       AND ($3::timestamptz IS NULL OR s.created_at >= $3)
       AND ($4::timestamptz IS NULL OR s.created_at < $4)";

const COLLECTED: &str = "p.status IN ('succeeded', 'partially_refunded', 'refunded')";

fn method_totals_sql(filter: &str) -> String {
    return format!(
        "SELECT pm.id AS payment_method_id, COALESCE(pm.name, p.method::text) AS name, p.method::text AS kind,
                COUNT(*)::bigint AS count,
                COALESCE(SUM(p.amount_minor) FILTER (WHERE {collected}), 0)::bigint AS collected_minor,
                COALESCE(SUM(p.refunded_amount_minor), 0)::bigint AS refunded_minor,
                COALESCE(SUM(p.amount_minor) FILTER (WHERE p.status = 'reversed'), 0)::bigint AS reversed_minor,
                COALESCE(SUM(p.amount_minor) FILTER (WHERE p.status = 'pending'), 0)::bigint AS pending_minor,
                COALESCE(SUM(p.amount_minor) FILTER (WHERE p.status = 'failed'), 0)::bigint AS failed_minor,
                (COALESCE(SUM(p.amount_minor) FILTER (WHERE {collected}), 0)
                    - COALESCE(SUM(p.refunded_amount_minor), 0))::bigint AS net_minor
         FROM atlas.payments p
         JOIN atlas.sales s ON s.id = p.sale_id
         LEFT JOIN atlas.payment_methods pm ON pm.id = p.payment_method_id
         WHERE {filter}
         GROUP BY pm.id, pm.name, p.method
         ORDER BY name",
        collected = COLLECTED,
        filter = filter
    );
}

/**
 * FR-16 reconciliation over one store's sales, optionally narrowed to a shift or a time window (by when the sale was rung up).
 * Payments and sales are summed independently over the same set of sales, so a bug that updates one side without the other shows up as variance instead of cancelling out.
 */
pub async fn reconcile(conn: &mut PgConnection, query: &ReconciliationQuery) -> Result<Reconciliation, ApiError> {
    let by_method = sqlx::query_as::<_, MethodTotals>(&method_totals_sql(SALE_FILTER))
        .bind(query.store_id)
        .bind(query.shift_id)
        .bind(query.from)
        .bind(query.to)
        .fetch_all(&mut *conn)
        .await?;

    let sales = sqlx::query_as::<_, SalesTotals>(&format!(
        "SELECT COUNT(*) FILTER (WHERE s.status = 'completed')::bigint AS completed_count,
                COUNT(*) FILTER (WHERE s.status = 'awaiting_payment')::bigint AS awaiting_payment_count,
                COUNT(*) FILTER (WHERE s.status = 'voided')::bigint AS voided_count,
                COALESCE(SUM(s.total_amount_minor + s.rounding_adjustment_minor) FILTER (WHERE s.status = 'completed'), 0)::bigint AS due_minor,
                COALESCE(SUM(s.amount_paid_minor), 0)::bigint AS amount_paid_minor,
                COALESCE(SUM(GREATEST(s.total_amount_minor + s.rounding_adjustment_minor - s.amount_paid_minor, 0))
                    FILTER (WHERE s.status = 'completed'), 0)::bigint AS outstanding_minor,
                COALESCE(SUM(s.amount_refunded_minor), 0)::bigint AS refunded_minor
         FROM atlas.sales s
         WHERE {}",
        SALE_FILTER
    ))
    .bind(query.store_id)
    .bind(query.shift_id)
    .bind(query.from)
    .bind(query.to)
    .fetch_one(&mut *conn)
    .await?;

    let non_payment_refunds_minor = non_payment_refunds(conn, query).await?;

    let discrepancies: Vec<Uuid> = sqlx::query_scalar(&format!(
        "SELECT s.id
         FROM atlas.sales s
         CROSS JOIN LATERAL (
             SELECT COALESCE(SUM(p.amount_minor) FILTER (WHERE {collected}), 0) AS collected
             FROM atlas.payments p WHERE p.sale_id = s.id
         ) pay
         WHERE {filter}
           AND (s.amount_paid_minor <> pay.collected
                OR (s.status = 'completed' AND s.payment_status = 'paid'
                    AND s.amount_paid_minor < s.total_amount_minor + s.rounding_adjustment_minor)
                OR s.amount_refunded_minor <> (SELECT COALESCE(SUM(r.amount_minor), 0) FROM atlas.refunds r
                                               WHERE r.sale_id = s.id AND r.status = 'executed'))
         ORDER BY s.created_at",
        collected = COLLECTED,
        filter = SALE_FILTER
    ))
    .bind(query.store_id)
    .bind(query.shift_id)
    .bind(query.from)
    .bind(query.to)
    .fetch_all(&mut *conn)
    .await?;

    let payments_net_minor = by_method.iter().map(|m| m.net_minor).sum::<i64>() - non_payment_refunds_minor;
    let sales_net_minor = sales.amount_paid_minor - sales.refunded_minor;
    return Ok(Reconciliation {
        store_id: query.store_id,
        shift_id: query.shift_id,
        from: query.from,
        to: query.to,
        by_method,
        variance_minor: payments_net_minor - sales_net_minor,
        payments_net_minor,
        sales_net_minor,
        non_payment_refunds_minor,
        sales,
        discrepancies,
    });
}

/// Refunds that never touched a payment row (to store credit, or out through a different method), so the payments side can account for them.
async fn non_payment_refunds(conn: &mut PgConnection, query: &ReconciliationQuery) -> Result<i64, ApiError> {
    return sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(r.amount_minor), 0)::bigint
         FROM atlas.refunds r JOIN atlas.sales s ON s.id = r.sale_id
         WHERE r.payment_id IS NULL AND r.status = 'executed' AND {}",
        SALE_FILTER
    ))
    .bind(query.store_id)
    .bind(query.shift_id)
    .bind(query.from)
    .bind(query.to)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * What should be in the drawer for a shift: the opening float plus cash taken in the shift, less cash refunded out of it.
 * Cash payments store the amount applied to the sale, change already excluded, so a N10,000 note for a N7,500 sale adds N7,500.
 */
pub async fn shift_cash_summary(conn: &mut PgConnection, shift_id: Uuid) -> Result<ShiftCashSummary, ApiError> {
    let (opening, counted): (i64, Option<i64>) =
        sqlx::query_as("SELECT opening_cash_amount_minor, closing_cash_amount_minor FROM atlas.shifts WHERE id = $1")
            .bind(shift_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| ApiError::not_found("Shift not found."))?;

    let by_method = sqlx::query_as::<_, MethodTotals>(&method_totals_sql("p.shift_id = $1"))
        .bind(shift_id)
        .fetch_all(&mut *conn)
        .await?;
    let cash_collected: i64 = by_method.iter().filter(|m| m.kind == "cash").map(|m| m.collected_minor).sum();
    let cash_refunded: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(r.amount_minor), 0)::bigint
         FROM atlas.refunds r
         JOIN atlas.payment_methods pm ON pm.id = r.payment_method_id
         WHERE r.shift_id = $1 AND r.status = 'executed' AND pm.kind = 'cash'",
    )
    .bind(shift_id)
    .fetch_one(&mut *conn)
    .await?;
    let expected = opening + cash_collected - cash_refunded;

    return Ok(ShiftCashSummary {
        opening_cash_minor: opening,
        cash_collected_minor: cash_collected,
        cash_refunded_minor: cash_refunded,
        expected_cash_minor: expected,
        counted_cash_minor: counted,
        variance_minor: counted.map(|c| c - expected),
        by_method,
    });
}
