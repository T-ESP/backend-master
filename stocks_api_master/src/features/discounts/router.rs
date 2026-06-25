use axum::{
    routing::{delete, get, post, put},
    Router,
};

use super::handlers::{
    check_discounts, create_discount, delete_discount, get_discount_by_id, get_discount_orders,
    get_discounts, get_order_discounts, update_discount,
};

pub fn discount_routes() -> Router {
    Router::new()
        .route("/", get(get_discounts).post(create_discount))
        .route("/check", post(check_discounts))
        .route("/:id", get(get_discount_by_id).put(update_discount).delete(delete_discount))
        .route("/:id/orders", get(get_discount_orders))
        .route("/order/:order_id/applied", get(get_order_discounts))
}
