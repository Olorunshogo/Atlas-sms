//! Integration coverage for `roles::service::assert_permission` and `assert_grantable` - the single generic gate every handler in the backend calls, and the containment check that makes the dynamic role system safe.
//! Nothing else in the system has meaningful test coverage today; this is the highest-leverage place to start, since a regression here silently breaks every module's access control at once.
//!
//! Uses `#[sqlx::test]`, which creates and migrates a fresh, isolated database per test - requires a plain `DATABASE_URL` env var (not `ATLAS_DATABASE_URL`) pointing at a Postgres server with rights to create databases, e.g. the same superuser `docker-compose.atlas.yml` sets up (`postgres://atlas:atlas@localhost:5434/atlas`).

use atlas_backend::roles;
use sqlx::PgPool;
use uuid::Uuid;

// === Fixtures

async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.users (id, full_name, email, password_hash, is_email_verified)
         VALUES ($1, 'Test User', $2, 'not-a-real-hash', true)",
    )
    .bind(id)
    .bind(format!("user-{}@example.com", id))
    .execute(pool)
    .await
    .unwrap();
    return id;
}

async fn seed_tenant(pool: &PgPool, created_by: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.tenants (id, name, created_by_user_id) VALUES ($1, 'Test Tenant', $2)")
        .bind(id)
        .bind(created_by)
        .execute(pool)
        .await
        .unwrap();
    return id;
}

async fn seed_store(pool: &PgPool, tenant_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.stores (id, tenant_id, name, slug) VALUES ($1, $2, 'Test Store', $3)")
        .bind(id)
        .bind(tenant_id)
        .bind(format!("store-{}", id))
        .execute(pool)
        .await
        .unwrap();
    return id;
}

/**
 * `permissions_sql_array` is a trusted literal like `"ARRAY['sale.create', 'stock.view']"` - interpolated directly rather than bound, since binding a Postgres enum array needs a Rust type that maps to `atlas.permission[]` that this codebase doesn't otherwise need.
 * Test-only, never fed anything but a hand-written literal.
 */
async fn seed_role(pool: &PgPool, tenant_id: Option<Uuid>, permissions_sql_array: &str) -> Uuid {
    let id = Uuid::now_v7();
    let query = format!(
        "INSERT INTO atlas.roles (id, tenant_id, name, permissions, is_system)
         VALUES ($1, $2, $3, {}::atlas.permission[], false)",
        permissions_sql_array
    );
    sqlx::query(&query)
        .bind(id)
        .bind(tenant_id)
        .bind(format!("role-{}", id))
        .execute(pool)
        .await
        .unwrap();
    return id;
}

#[allow(clippy::too_many_arguments)]
async fn seed_grant(
    pool: &PgPool,
    user_id: Uuid,
    role_id: Uuid,
    scope: &str,
    tenant_id: Option<Uuid>,
    store_id: Option<Uuid>,
    status: &str,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.role_grants
            (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
         VALUES ($1, $2, $3, $4::atlas.role_scope, $5, $6, $7::atlas.role_grant_status, $2)",
    )
    .bind(id)
    .bind(user_id)
    .bind(role_id)
    .bind(scope)
    .bind(tenant_id)
    .bind(store_id)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    return id;
}

async fn seed_store_group_grant(pool: &PgPool, role_grant_id: Uuid, store_id: Uuid) {
    sqlx::query("INSERT INTO atlas.store_group_grants (role_grant_id, store_id) VALUES ($1, $2)")
        .bind(role_grant_id)
        .bind(store_id)
        .execute(pool)
        .await
        .unwrap();
}

// === assert_permission

#[sqlx::test]
async fn tenant_scope_grant_permits_matching_tenant(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['sale.create']").await;
    seed_grant(&pool, user_id, role_id, "tenant", Some(tenant_id), None, "active").await;

    let result = roles::service::assert_permission(&mut conn, user_id, tenant_id, None, "sale.create").await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn tenant_scope_grant_denies_a_different_tenant(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let granted_tenant = seed_tenant(&pool, user_id).await;
    let other_tenant = seed_tenant(&pool, user_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['sale.create']").await;
    seed_grant(&pool, user_id, role_id, "tenant", Some(granted_tenant), None, "active").await;

    let result = roles::service::assert_permission(&mut conn, user_id, other_tenant, None, "sale.create").await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn store_scope_grant_permits_the_matching_store(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let store_id = seed_store(&pool, tenant_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['stock.adjust']").await;
    seed_grant(&pool, user_id, role_id, "store", Some(tenant_id), Some(store_id), "active").await;

    let result =
        roles::service::assert_permission(&mut conn, user_id, tenant_id, Some(store_id), "stock.adjust").await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn store_scope_grant_denies_a_different_store(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let granted_store = seed_store(&pool, tenant_id).await;
    let other_store = seed_store(&pool, tenant_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['stock.adjust']").await;
    seed_grant(&pool, user_id, role_id, "store", Some(tenant_id), Some(granted_store), "active").await;

    let result =
        roles::service::assert_permission(&mut conn, user_id, tenant_id, Some(other_store), "stock.adjust").await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn platform_scope_grant_permits_any_tenant_and_store(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let some_tenant = seed_tenant(&pool, user_id).await;
    let some_store = seed_store(&pool, some_tenant).await;
    let role_id = seed_role(&pool, None, "ARRAY['tenant.manage']").await;
    seed_grant(&pool, user_id, role_id, "platform", None, None, "active").await;

    let result = roles::service::assert_permission(
        &mut conn,
        user_id,
        some_tenant,
        Some(some_store),
        "tenant.manage",
    )
    .await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn a_role_missing_the_permission_denies_even_with_a_matching_scope(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    // The role holds a real permission, just not the one being checked.
    let role_id = seed_role(&pool, None, "ARRAY['stock.view']").await;
    seed_grant(&pool, user_id, role_id, "tenant", Some(tenant_id), None, "active").await;

    let result = roles::service::assert_permission(&mut conn, user_id, tenant_id, None, "tenant.manage").await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn a_pending_grant_does_not_grant_access(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['sale.create']").await;
    seed_grant(&pool, user_id, role_id, "tenant", Some(tenant_id), None, "pending").await;

    let result = roles::service::assert_permission(&mut conn, user_id, tenant_id, None, "sale.create").await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn a_revoked_grant_does_not_grant_access(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['sale.create']").await;
    seed_grant(&pool, user_id, role_id, "tenant", Some(tenant_id), None, "revoked").await;

    let result = roles::service::assert_permission(&mut conn, user_id, tenant_id, None, "sale.create").await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn a_store_group_grant_extends_a_store_scope_grant_to_another_store(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;
    let home_store = seed_store(&pool, tenant_id).await;
    let extra_store = seed_store(&pool, tenant_id).await;
    let role_id = seed_role(&pool, None, "ARRAY['report.view']").await;
    let grant_id =
        seed_grant(&pool, user_id, role_id, "store", Some(tenant_id), Some(home_store), "active").await;
    seed_store_group_grant(&pool, grant_id, extra_store).await;

    // Not the grant's own store - only reachable via the store_group_grants row.
    let result =
        roles::service::assert_permission(&mut conn, user_id, tenant_id, Some(extra_store), "report.view").await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn a_user_with_no_grant_at_all_is_denied(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let user_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, user_id).await;

    let result = roles::service::assert_permission(&mut conn, user_id, tenant_id, None, "sale.create").await;
    assert!(result.is_err());
}

// === assert_grantable

#[sqlx::test]
async fn a_granter_with_a_superset_of_permissions_can_grant_the_role(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let granter_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, granter_id).await;
    let granter_role = seed_role(&pool, None, "ARRAY['staff.invite', 'sale.create', 'stock.view']").await;
    seed_grant(&pool, granter_id, granter_role, "tenant", Some(tenant_id), None, "active").await;
    let target_role = seed_role(&pool, None, "ARRAY['sale.create']").await;

    let result = roles::service::assert_grantable(&mut conn, granter_id, tenant_id, None, target_role).await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn a_granter_missing_a_permission_the_target_role_has_cannot_grant_it(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let granter_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, granter_id).await;
    let granter_role = seed_role(&pool, None, "ARRAY['staff.invite', 'sale.create']").await;
    seed_grant(&pool, granter_id, granter_role, "tenant", Some(tenant_id), None, "active").await;
    // Target role needs tenant.manage, which the granter does not hold.
    let target_role = seed_role(&pool, None, "ARRAY['sale.create', 'tenant.manage']").await;

    let result = roles::service::assert_grantable(&mut conn, granter_id, tenant_id, None, target_role).await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn effective_permissions_union_across_more_than_one_grant(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let granter_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, granter_id).await;
    let store_id = seed_store(&pool, tenant_id).await;

    // Neither grant alone covers the target role's permissions - only their union does.
    let tenant_role = seed_role(&pool, None, "ARRAY['staff.invite']").await;
    seed_grant(&pool, granter_id, tenant_role, "tenant", Some(tenant_id), None, "active").await;
    let store_role = seed_role(&pool, None, "ARRAY['stock.adjust']").await;
    seed_grant(&pool, granter_id, store_role, "store", Some(tenant_id), Some(store_id), "active").await;

    let target_role = seed_role(&pool, None, "ARRAY['stock.adjust']").await;

    let result =
        roles::service::assert_grantable(&mut conn, granter_id, tenant_id, Some(store_id), target_role).await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn granting_an_unknown_role_id_fails(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let granter_id = seed_user(&pool).await;
    let tenant_id = seed_tenant(&pool, granter_id).await;
    let granter_role = seed_role(&pool, None, "ARRAY['staff.invite']").await;
    seed_grant(&pool, granter_id, granter_role, "tenant", Some(tenant_id), None, "active").await;

    let result =
        roles::service::assert_grantable(&mut conn, granter_id, tenant_id, None, Uuid::now_v7()).await;
    assert!(result.is_err());
}
