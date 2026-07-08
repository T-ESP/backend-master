use axum::{
    routing::{get, post, put, delete},
    Router,
};

use super::handlers::{
    get_orders, create_order, get_order_by_id, update_order, delete_order,
    get_order_items, get_orders_by_user, get_order_stats, send_receipt
};

pub fn order_routes() -> Router {
    Router::new()
        .route("/", get(get_orders).post(create_order))
        .route("/:id", get(get_order_by_id).put(update_order).delete(delete_order))
        .route("/:id/items", get(get_order_items))
        .route("/:id/receipt", post(send_receipt))
        .route("/user/:user_id", get(get_orders_by_user))
        .route("/stats", get(get_order_stats))
}
