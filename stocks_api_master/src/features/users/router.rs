use axum::{Router, routing::get};
use super::handlers;

pub fn user_routes() -> Router {
    Router::new()
        .route("/", get(handlers::get_users).post(handlers::create_user))
        .route("/fidelity/:code", get(handlers::get_user_by_fidelity_code))
        .route(
            "/:id",
            get(handlers::get_user_by_id)
                .put(handlers::update_user)
                .delete(handlers::delete_user),
        )
}
