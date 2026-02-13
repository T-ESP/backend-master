use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::common::security::{self, Claims};

pub async fn require_auth(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {

    let authorization = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = authorization
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match security::validate_jwt(token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
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

pub fn get_claims(req: &Request<Body>) -> Option<&Claims> {
    req.extensions().get::<Claims>()
}
