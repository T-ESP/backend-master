use axum::{routing::get, Router};

use super::handlers;

pub fn staff_routes() -> Router {
    Router::new()
        .route("/", get(handlers::get_staff).post(handlers::create_staff))
        .route(
            "/:id",
            get(handlers::get_staff_by_id)
                .put(handlers::update_staff)
                .delete(handlers::delete_staff),
        )
}
