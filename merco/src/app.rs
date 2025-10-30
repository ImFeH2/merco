use crate::tasks::TaskManager;
use crate::{handlers, strategy::StrategyManager};
use axum::routing::delete;
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Clone)]
pub struct AppState {
    pub task_manager: TaskManager,
    pub strategy_manager: StrategyManager,
    pub db_pool: PgPool,
    pub shutdown_token: CancellationToken,
}

pub fn create_app(db_pool: PgPool, shutdown_token: CancellationToken) -> Router {
    let task_manager = TaskManager::new();
    let strategy_manager = StrategyManager::new().expect("Failed to create StrategyManager");

    let state = AppState {
        task_manager,
        strategy_manager,
        db_pool,
        shutdown_token,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(handlers::info::check))
        .route("/error", get(handlers::info::error))
        .route("/exchanges", get(handlers::info::list_exchanges))
        .route("/symbols", get(handlers::info::list_symbols))
        .route("/timeframes", get(handlers::info::list_timeframes))
        .route("/tasks", get(handlers::tasks::get_all_tasks))
        .route("/tasks/{id}", get(handlers::tasks::get_task))
        .route("/tasks/stream", get(handlers::tasks::stream_tasks))
        .route("/tasks/fetch", post(handlers::tasks::create_fetch_task))
        .route("/candles", get(handlers::candles::get_candles))
        .route("/strategy/add", post(handlers::strategy::add_strategy))
        .route("/strategy/backtest", post(handlers::strategy::backtest))
        .route("/strategy/source/get", get(handlers::source::get_source))
        .route("/strategy/source/save", post(handlers::source::save_source))
        .route(
            "/strategy/source/delete",
            get(handlers::source::delete_source),
        )
        .route("/strategy/source/move", get(handlers::source::move_source))
        .layer(cors)
        .with_state(state)
}
