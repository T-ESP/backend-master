use axum::{
    routing::get,
    Router,
};

use super::handlers::{get_loyalty_config, update_loyalty_config, get_user_loyalty};

pub fn loyalty_routes() -> Router {
    Router::new()
        .route("/config", get(get_loyalty_config).put(update_loyalty_config))
        .route("/users/:user_id", get(get_user_loyalty))
}
