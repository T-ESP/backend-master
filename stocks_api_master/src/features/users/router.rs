use axum::{Router, routing::{get, post, put, delete}};
use super::handlers;

pub fn user_routes() -> Router {
    Router::new()
        .route("/", get(handlers::get_users).post(handlers::create_user))
        .route("/:id", put(handlers::update_user).delete(handlers::delete_user))
}
