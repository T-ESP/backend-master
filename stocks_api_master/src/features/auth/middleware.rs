use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::security::Claims;

pub fn get_claims(req: &Request<Body>) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

pub async fn require_tenant_match_subdomain(
    State(pool): State<PgPool>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {

    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Les platform_admin passent toujours
    if claims.role == "platform_admin" {
        return Ok(next.run(req).await);
    }

    // Vérifier qu'on a un commerce_id
    let commerce_id = claims
        .commerce_id
        .ok_or(StatusCode::FORBIDDEN)?;

    // Récupérer le host (ex: test.stock-s.fr)
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Extraire le slug (partie avant le premier point)
    let slug = host
        .split('.')
        .next()
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Vérifier en base que le slug correspond au commerce_id
    let row = sqlx::query(
        "SELECT id FROM commerces WHERE slug = $1"
    )
        .bind(slug)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(row) = row {
        let db_id: Uuid = row.get("id");

        if db_id == commerce_id {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::FORBIDDEN)
}

pub async fn require_auth(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = crate::common::security::validate_jwt(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut req = req;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn require_platform_admin(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if claims.role != "platform_admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
