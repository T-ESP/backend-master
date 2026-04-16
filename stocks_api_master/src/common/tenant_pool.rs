use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct TenantPoolManager {
    master_pool: PgPool,
    database_url: String,
    pools: Arc<RwLock<HashMap<Uuid, PgPool>>>,
}

impl TenantPoolManager {
    pub fn new(master_pool: PgPool, database_url: String) -> Self {
        Self {
            master_pool,
            database_url,
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn base_db_url(&self) -> Option<String> {
        let last_slash = self.database_url.rfind('/')?;
        Some(self.database_url[..last_slash].to_string())
    }

    pub async fn get_pool(&self, commerce_id: Uuid) -> Result<PgPool, StatusCode> {
        // Check cache first
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&commerce_id) {
                return Ok(pool.clone());
            }
        }

        // Look up db_name in master database
        let row = sqlx::query_scalar::<_, String>(
            "SELECT db_name FROM commerces WHERE id = $1 AND status = 'active'"
        )
        .bind(commerce_id)
        .fetch_optional(&self.master_pool)
        .await
        .map_err(|e| {
            eprintln!("Failed to look up commerce {}: {}", commerce_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let db_name = row.ok_or(StatusCode::NOT_FOUND)?;

        let base_url = self.base_db_url().ok_or_else(|| {
            eprintln!("Invalid DATABASE_URL format");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let tenant_url = format!("{}/{}", base_url, db_name);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&tenant_url)
            .await
            .map_err(|e| {
                eprintln!("Failed to connect to tenant db {}: {}", db_name, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Cache the pool
        {
            let mut pools = self.pools.write().await;
            pools.insert(commerce_id, pool.clone());
        }

        Ok(pool)
    }
}

/// Middleware that resolves the tenant pool from the `commerce_id` path parameter
/// and injects it as an Extension<PgPool> for downstream handlers.
pub async fn resolve_tenant_pool(
    State(manager): State<TenantPoolManager>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();

    // Extract commerce_id from path: /api/{commerce_id}/...
    let commerce_id = extract_commerce_id_from_path(&path)?;

    let pool = manager.get_pool(commerce_id).await?;

    req.extensions_mut().insert(pool);

    Ok(next.run(req).await)
}

fn extract_commerce_id_from_path(path: &str) -> Result<Uuid, StatusCode> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Expected: ["api", "{commerce_id}", "products", ...]
    if segments.len() < 2 || segments[0] != "api" {
        return Err(StatusCode::BAD_REQUEST);
    }

    Uuid::parse_str(segments[1]).map_err(|_| StatusCode::BAD_REQUEST)
}
