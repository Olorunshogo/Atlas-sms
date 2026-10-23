mod common;

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

/**
 * Two regressions in one: roles are cluster-wide, so a second database on
 * the same server used to fail at `CREATE ROLE atlas_app`; and re-running
 * migrations on an already-migrated database must be a clean no-op, which
 * used to replay everything when the admin role shared the schema's name.
 */
#[tokio::test]
async fn migrations_apply_to_a_second_database_and_rerun_cleanly() {
    let url = std::env::var("ATLAS_TEST_DATABASE_ADMIN_URL")
        .expect("ATLAS_TEST_DATABASE_ADMIN_URL must be set - see the README's \"Running the tests\" section");
    let base = PgConnectOptions::from_str(&url)
        .expect("parse admin url")
        .options([("search_path", "public")]);

    // Make sure the shared test database (and so the cluster-wide role) exists first.
    let first = common::world().await;
    drop(first);

    let name = format!("atlas_migrations_{}", Uuid::now_v7().simple());
    let server = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(base.clone())
        .await
        .expect("connect to server");
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&server)
        .await
        .expect("create scratch database");

    let scratch = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(base.clone().database(&name))
        .await
        .expect("connect to scratch database");
    let migrator = sqlx::migrate!("./migrations");

    let outcome = async {
        migrator.run(&scratch).await.map_err(|e| format!("first run: {e}"))?;
        migrator.run(&scratch).await.map_err(|e| format!("second run: {e}"))?;
        let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&scratch)
            .await
            .map_err(|e| e.to_string())?;
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'atlas' AND table_name = '_sqlx_migrations'",
        )
        .fetch_one(&scratch)
        .await
        .map_err(|e| e.to_string())?;
        Ok::<_, String>((applied, tables))
    }
    .await;

    scratch.close().await;
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&server)
        .await
        .expect("drop scratch database");

    let (applied, stray_tables) = outcome.expect("migrations");
    assert_eq!(
        applied as usize,
        migrator.iter().count(),
        "every migration recorded once"
    );
    assert_eq!(
        stray_tables, 0,
        "bookkeeping must not land in the atlas schema"
    );
}
