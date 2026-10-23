// Format strings use positional args, not inline captures, throughout this
// crate - see the coding-assistant skill's locked rule.
#![allow(clippy::uninlined_format_args)]

use atlas_backend::{api_router, app, build_state};

#[tokio::main]
async fn main() {
    // ATLAS_PORT for `nx dev` (shared root .env, must not collide with
    // atlas-backend's own PORT); PORT for docker-compose.atlas.yml,
    // which sets it in its own isolated container environment.
    let port = std::env::var("ATLAS_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "5755".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("failed to bind port");

    let state = build_state().await;
    let router = app().merge(api_router(state));

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}
