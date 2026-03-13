use axum::{Router, routing::{get, post, put, delete}};
use super::handlers;

pub fn suppliers_routes() -> Router {
    Router::new()
        .route("/", get(handlers::get_all_suppliers).post(handlers::create_supplier))
        .route("/search", get(handlers::get_supplier_by_email))  // ← Nouvelle route
        .route("/:id",
               get(handlers::get_supplier_by_id)
                   .put(handlers::update_supplier)
                   .delete(handlers::delete_supplier)
        )
}
