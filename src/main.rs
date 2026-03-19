use std::sync::Arc;

use axum::routing::post;
use axum::Router;
use tower_http::cors::CorsLayer;

mod auth;
mod db;
mod error;
mod models;

#[tokio::main]
async fn main() {
    let pool = db::init_pool().await;

    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into());

    let state = auth::AppState {
        pool,
        jwt_secret: Arc::new(jwt_secret),
    };

    let app = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh-token", post(auth::refresh_token))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
