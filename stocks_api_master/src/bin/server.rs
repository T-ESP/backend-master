use anyhow::Result;
use axum::{http::Method, middleware::{from_fn, from_fn_with_state}, routing::get, Router};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tokio::net::TcpListener;

use stocks_api::{common::security, features};
use stocks_api::openapi::ApiDoc;

use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL")?;
    let jwt_secret = env::var("JWT_SECRET")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await?;

    security::init_jwt_secret(jwt_secret)?;

    async fn health() -> &'static str {
        "OK"
    }

    // 🔐 Routes protégées MASTER (platform admin uniquement)
    let protected_master_routes = Router::new()
        .nest(
            "/admin/tenants",
            features::tenants::router::tenant_routes(pool.clone()),
        )
        .layer(from_fn(features::auth::middleware::require_platform_admin))
        .layer(from_fn(features::auth::middleware::require_auth));

    // 🔐 Routes protégées TENANT (commerce)
    let protected_tenant_routes = Router::new()
        .nest(
            "/api",
            features::tenants::router::tenant_routes(pool.clone()),
        )
        .layer(from_fn_with_state(
            pool.clone(),
            features::auth::middleware::require_tenant_match_subdomain,
        ))
        .layer(from_fn(features::auth::middleware::require_auth));

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),
            "http://localhost:5174".parse().unwrap(),
            "https://stock-s.fr".parse().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .nest("/auth", features::auth::router::auth_routes(pool.clone()))
        .merge(protected_master_routes)
        .merge(protected_tenant_routes)
        .layer(cors);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;

    println!("API disponible sur http://0.0.0.0:8080");
    println!("Swagger UI disponible sur http://0.0.0.0:8080/swagger-ui");

    axum::serve(listener, app).await?;

    Ok(())
}
