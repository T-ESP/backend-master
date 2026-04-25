use axum::{
    routing::{get, post},
    Router,
};

use super::handlers::{
    get_loyalty_config, update_loyalty_config,
    get_user_loyalty, get_user_discount, adjust_user_points,
};

pub fn loyalty_routes() -> Router {
    Router::new()
        .route("/config", get(get_loyalty_config).put(update_loyalty_config))
        .route("/users/:user_id", get(get_user_loyalty))
        .route("/users/:user_id/discount", get(get_user_discount))
        .route("/users/:user_id/points", post(adjust_user_points))
}
