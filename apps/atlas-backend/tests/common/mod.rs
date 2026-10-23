#![allow(dead_code)]

use std::str::FromStr;
use std::sync::Arc;

use atlas_backend::{api_router, issue_access_token, AppState, LogEmailProvider};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// === Setup

/// Tests run in parallel inside one process; migrating concurrently races.
static MIGRATED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

const JWT_SECRET: &[u8] = b"integration-test-secret";
const OWNER_ROLE_ID: &str = "00000000-0000-7000-8000-000000000001";
pub const ADMIN_ROLE_ID: &str = "00000000-0000-7000-8000-000000000002";
pub const MANAGER_ROLE_ID: &str = "00000000-0000-7000-8000-000000000003";
pub const ACCOUNTANT_ROLE_ID: &str = "00000000-0000-7000-8000-000000000006";
pub const CASHIER_ROLE_ID: &str = "00000000-0000-7000-8000-000000000004";

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!(
            "{name} must be set to run the integration tests - see the README's \
             \"Running the tests\" section. They need a real Postgres, not a mock."
        )
    })
}

/**
 * One tenant with three stores, a product, a price and an owner, created
 * through the superuser connection. Every test builds its own, so tests
 * share a database but never rows, and can run in parallel.
 */
pub struct World {
    pub app: Router,
    /// Superuser connection, bypasses RLS: for fixtures and for asserting on raw tables.
    pub admin: PgPool,
    pub tenant_id: Uuid,
    pub owner_id: Uuid,
    pub token: String,
    pub stores: [Uuid; 3],
    pub shifts: [Uuid; 3],
    pub rice: Uuid,
    pub piece_unit: Uuid,
}

/**
 * A tenant with an owner and nothing else: no stores, units, products or
 * prices. For tests that must build everything through the API.
 */
pub async fn bare_world() -> World {
    /*
     * Pinned to `public` because the admin role is conventionally named
     * `atlas`, the same as the schema: with the default search_path
     * ("$user", public) sqlx's bookkeeping table lands in the `atlas`
     * schema on the second run and every migration replays.
     */
    let admin_options = PgConnectOptions::from_str(&env("ATLAS_TEST_DATABASE_ADMIN_URL"))
        .expect("parse admin url")
        .options([("search_path", "public")]);
    let admin = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(admin_options)
        .await
        .expect("connect as admin");
    MIGRATED
        .get_or_init(|| async {
            sqlx::migrate!("./migrations")
                .run(&admin)
                .await
                .expect("run migrations");
        })
        .await;

    let pool = PgPoolOptions::new()
        .max_connections(12)
        .connect(&env("ATLAS_TEST_DATABASE_URL"))
        .await
        .expect("connect as atlas_api");
    let app = api_router(AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(JWT_SECRET.to_vec()),
    });

    let tenant_id = Uuid::now_v7();
    let owner_id = new_user(&admin).await;
    sqlx::query("INSERT INTO atlas.tenants (id, name, status, created_by_user_id) VALUES ($1, 'Test Tenant', 'active', $2)")
        .bind(tenant_id)
        .bind(owner_id)
        .execute(&admin)
        .await
        .unwrap();
    grant(
        &admin,
        owner_id,
        OWNER_ROLE_ID,
        "tenant",
        Some(tenant_id),
        None,
        owner_id,
    )
    .await;

    let token = token_for(owner_id, tenant_id);
    World {
        app,
        admin,
        tenant_id,
        owner_id,
        token,
        stores: [Uuid::nil(); 3],
        shifts: [Uuid::nil(); 3],
        rice: Uuid::nil(),
        piece_unit: Uuid::nil(),
    }
}

/**
 * `bare_world` plus three stores with open shifts, a piece unit and rice
 * priced at 150000 minor units per piece, inserted directly with SQL.
 */
pub async fn world() -> World {
    let mut world = bare_world().await;
    let admin = world.admin.clone();
    let tenant_id = world.tenant_id;
    let owner_id = world.owner_id;

    let mut stores = [Uuid::nil(); 3];
    let mut shifts = [Uuid::nil(); 3];
    for (i, name) in ["Location One", "Location Two", "Location Three"]
        .iter()
        .enumerate()
    {
        stores[i] = Uuid::now_v7();
        sqlx::query("INSERT INTO atlas.stores (id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)")
            .bind(stores[i])
            .bind(tenant_id)
            .bind(name)
            .bind(format!("location-{}", i + 1))
            .execute(&admin)
            .await
            .unwrap();
        shifts[i] = Uuid::now_v7();
        sqlx::query("INSERT INTO atlas.shifts (id, tenant_id, store_id, opened_by_user_id) VALUES ($1, $2, $3, $4)")
            .bind(shifts[i])
            .bind(tenant_id)
            .bind(stores[i])
            .bind(owner_id)
            .execute(&admin)
            .await
            .unwrap();
    }

    let piece_unit = new_unit(&admin, tenant_id, "piece").await;
    let product_type = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.product_types (id, tenant_id, name) VALUES ($1, $2, 'Grocery')")
        .bind(product_type)
        .bind(tenant_id)
        .execute(&admin)
        .await
        .unwrap();

    let rice = new_product(&admin, tenant_id, product_type, piece_unit, "Rice", 150_000).await;
    world.stores = stores;
    world.shifts = shifts;
    world.rice = rice;
    world.piece_unit = piece_unit;
    world
}

pub fn token_for(user_id: Uuid, tenant_id: Uuid) -> String {
    issue_access_token(user_id, tenant_id, JWT_SECRET).expect("issue token")
}

async fn new_user(admin: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.users (id, full_name, email, password_hash) VALUES ($1, 'Test User', $2, 'x')")
        .bind(id)
        .bind(format!("{id}@test.local"))
        .execute(admin)
        .await
        .unwrap();
    id
}

async fn grant(
    admin: &PgPool,
    user_id: Uuid,
    role_id: &str,
    scope: &str,
    tenant_id: Option<Uuid>,
    store_id: Option<Uuid>,
    granted_by: Uuid,
) {
    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
         VALUES ($1, $2, $3::uuid, $4::atlas.role_scope, $5, $6, 'active', $7)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(role_id)
    .bind(scope)
    .bind(tenant_id)
    .bind(store_id)
    .bind(granted_by)
    .execute(admin)
    .await
    .unwrap();
}

/** A cashier whose only grant is at one store. Returns their token. */
pub async fn store_cashier(world: &World, store_id: Uuid) -> String {
    store_user(world, store_id, CASHIER_ROLE_ID).await
}

/** A user holding `role_id` at one store only. Returns their token. */
pub async fn store_user(world: &World, store_id: Uuid, role_id: &str) -> String {
    let user_id = new_user(&world.admin).await;
    grant(
        &world.admin,
        user_id,
        role_id,
        "store",
        Some(world.tenant_id),
        Some(store_id),
        world.owner_id,
    )
    .await;
    token_for(user_id, world.tenant_id)
}

pub async fn new_unit(admin: &PgPool, tenant_id: Uuid, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.units_of_measure (id, tenant_id, name, abbreviation) VALUES ($1, $2, $3, $3)")
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .execute(admin)
        .await
        .unwrap();
    id
}

/** Inserts a product with a tenant-wide price rule (minor units per base unit). */
pub async fn new_product(
    admin: &PgPool,
    tenant_id: Uuid,
    product_type_id: Uuid,
    base_unit_id: Uuid,
    name: &str,
    price_minor: i64,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.products (id, tenant_id, product_type_id, base_unit_id, name) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(tenant_id)
        .bind(product_type_id)
        .bind(base_unit_id)
        .bind(name)
        .execute(admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO atlas.price_rules (id, tenant_id, product_id, unit_price_amount_minor) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(id)
        .bind(price_minor)
        .execute(admin)
        .await
        .unwrap();
    id
}

// === Requests

pub async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"));
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

impl World {
    pub async fn get(&self, uri: &str) -> (StatusCode, Value) {
        send(&self.app, Method::GET, uri, &self.token, None).await
    }

    pub async fn patch(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        send(&self.app, Method::PATCH, uri, &self.token, Some(body)).await
    }

    pub async fn delete(&self, uri: &str) -> (StatusCode, Value) {
        send(&self.app, Method::DELETE, uri, &self.token, None).await
    }

    pub async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        send(&self.app, Method::POST, uri, &self.token, Some(body)).await
    }

    /// Adds `product` to `store` with starting stock through the real endpoint.
    pub async fn seed_stock(&self, store: Uuid, product: Uuid, quantity: i64) {
        let (status, body) = self
            .post(
                "/api/v1/stock/inventory",
                json!({ "store_id": store, "product_id": product, "quantity": quantity }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "seed stock failed: {body}");
    }

    /// Rice at the three stores: 50 / 30 / 20.
    pub async fn seed_rice(&self) {
        for (store, quantity) in self.stores.iter().zip([50, 30, 20]) {
            self.seed_stock(*store, self.rice, quantity).await;
        }
    }

    pub async fn sell(
        &self,
        store_index: usize,
        product: Uuid,
        quantity: i64,
    ) -> (StatusCode, Value) {
        self.sell_lines(store_index, &[(product, quantity)]).await
    }

    pub async fn sell_lines(
        &self,
        store_index: usize,
        lines: &[(Uuid, i64)],
    ) -> (StatusCode, Value) {
        self.post("/api/v1/sales", self.sale_body(store_index, lines))
            .await
    }

    pub fn sale_body(&self, store_index: usize, lines: &[(Uuid, i64)]) -> Value {
        // `unit_price_amount_minor: 1` is deliberate: the server must ignore it.
        json!({
            "id": Uuid::now_v7(),
            "store_id": self.stores[store_index],
            "shift_id": self.shifts[store_index],
            "device_id": "till-1",
            "lines": lines.iter().map(|(product, quantity)| json!({
                "product_id": product,
                "unit_id": self.piece_unit,
                "quantity": quantity,
                "unit_price_amount_minor": 1,
            })).collect::<Vec<_>>(),
        })
    }

    pub fn transfer_body(&self, from: usize, to: usize, quantity: i64) -> Value {
        json!({
            "from_store_id": self.stores[from],
            "to_store_id": self.stores[to],
            "product_id": self.rice,
            "quantity": quantity,
            "reason": "Rebalancing",
        })
    }

    /// On-hand for a product at a store, read from the API.
    pub async fn on_hand(&self, store_index: usize, product: Uuid) -> Decimal {
        let uri = format!(
            "/api/v1/stock/on-hand?store_id={}&product_id={product}",
            self.stores[store_index]
        );
        let (status, body) = self.get(&uri).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["data"]
            .as_array()
            .unwrap()
            .first()
            .map(|row| dec(&row["on_hand"]))
            .unwrap_or(Decimal::ZERO)
    }

    /// Product total across stores, read from the API.
    pub async fn total(&self, product: Uuid) -> Decimal {
        let (status, body) = self.get(&format!("/api/v1/stock/products/{product}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        dec(&body["data"]["total_quantity"])
    }

    // == Raw table counts, straight from the database

    pub async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(&format!(
            "SELECT count(*) FROM atlas.{table} WHERE tenant_id = $1"
        ))
        .bind(self.tenant_id)
        .fetch_one(&self.admin)
        .await
        .unwrap()
    }

    pub async fn count_movements(&self, movement_type: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM atlas.stock_movements WHERE tenant_id = $1 AND movement_type = $2::atlas.stock_movement_type")
            .bind(self.tenant_id)
            .bind(movement_type)
            .fetch_one(&self.admin)
            .await
            .unwrap()
    }
}

// === Raw requests

/**
 * Like `send`, but the caller controls the exact body and content type, to
 * exercise malformed input that `serde_json::Value` cannot express.
 */
pub async fn send_raw(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    content_type: Option<&str>,
    body: &str,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if let Some(content_type) = content_type {
        request = request.header("content-type", content_type);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .map(|value| value.to_str().unwrap().to_string());
    let bytes = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, content_type, bytes)
}

impl World {
    /**
     * Every stock balance must equal the sum of its ledger rows. Called at
     * the end of tests that mix operations, to catch the two drifting.
     */
    pub async fn assert_ledger_matches_balances(&self) {
        let drifted: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM atlas.stock_levels sl
             WHERE sl.tenant_id = $1
               AND sl.quantity <> (SELECT COALESCE(SUM(m.quantity), 0) FROM atlas.stock_movements m
                                   WHERE m.store_id = sl.store_id AND m.product_id = sl.product_id)",
        )
        .bind(self.tenant_id)
        .fetch_one(&self.admin)
        .await
        .unwrap();
        assert_eq!(drifted, 0, "stock_levels drifted from stock_movements");
    }
}

// === Helpers

pub fn dec(value: &Value) -> Decimal {
    match value {
        Value::String(text) => Decimal::from_str(text).unwrap(),
        Value::Number(number) => Decimal::from_str(&number.to_string()).unwrap(),
        other => panic!("not a decimal: {other}"),
    }
}

pub fn d(value: i64) -> Decimal {
    Decimal::from(value)
}
