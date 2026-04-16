use anyhow::Result;
use axum::{http::{Method, header, HeaderValue}, middleware::{from_fn, from_fn_with_state}, routing::get, Router};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tokio::net::TcpListener;

use stocks_api::{common::security, features};
use stocks_api::common::tenant_pool::{TenantPoolManager, resolve_tenant_pool};
use stocks_api::openapi::ApiDoc;

use tower_http::cors::CorsLayer;
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

    let tenant_pool_manager = TenantPoolManager::new(pool.clone(), db_url.clone());

    // 🔐 Routes protégées MASTER (platform admin uniquement)
    let protected_master_routes = Router::new()
        .nest(
            "/admin/tenants",
            features::tenants::router::tenant_admin_routes(pool.clone()),
        )
        .layer(from_fn(features::auth::middleware::require_platform_admin))
        .layer(from_fn(features::auth::middleware::require_auth));

    // 🔐 Routes protégées TENANT (commerce)
    let protected_tenant_routes = Router::new()
        .nest("/api/:commerce_id/products", features::products::router::product_routes())
        .nest("/api/:commerce_id/suppliers", features::suppliers::router::suppliers_routes())
        .nest("/api/:commerce_id/stocks", features::stocks::router::stock_routes())
        .nest("/api/:commerce_id/orders", features::orders::router::order_routes())
        .nest("/api/:commerce_id/sales", features::sales::router::sales_routes())
        .nest("/api/:commerce_id/users", features::users::router::user_routes())
        .nest("/api/:commerce_id/ai/insights", features::ai_insights::router::ai_insights_routes())
        .nest("/api/:commerce_id/alerts", features::alerts::router::alert_routes())
        .nest("/api/:commerce_id/ai/predictions", features::ai_predictions::router::ai_prediction_routes())
        .nest("/api/:commerce_id/kpis", features::global_kpis::router::global_kpis_routes())
        .nest("/api/:commerce_id/restocks", features::restocks::router::restock_routes())
        .layer(from_fn_with_state(tenant_pool_manager, resolve_tenant_pool))
        .layer(from_fn(features::auth::middleware::require_auth));

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            |origin: &HeaderValue, _| {
                origin
                    .to_str()
                    .map(|s| {
                        s == "http://localhost:4200"
                            || s == "http://localhost:5173"
                            || s == "http://localhost:5174"
                            || s == "https://stock-s.fr"
                            || s.ends_with(".stock-s.fr")
                    })
                    .unwrap_or(false)
            },
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .route("/health", get(health))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .nest("/auth", features::auth::router::auth_routes(pool.clone()))
        // 🌐 Route publique — vérification tenant pour Angular
        .merge(features::tenants::router::tenant_public_routes(pool.clone()))
        .merge(protected_master_routes)
        .merge(protected_tenant_routes)
        .layer(cors);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;

    println!("API disponible sur http://0.0.0.0:8080");
    println!("Swagger UI disponible sur http://0.0.0.0:8080/swagger-ui");

    axum::serve(listener, app).await?;

    Ok(())
}
