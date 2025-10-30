use crate::{
    app::AppState,
    errors::ApiResult,
    models::Timeframe,
    strategy::{StrategyContext, context::BacktestContext},
};
use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct AddStrategyRequest {
    pub name: String,
}

pub async fn add_strategy(
    State(state): State<AppState>,
    Json(request): Json<AddStrategyRequest>,
) -> ApiResult<()> {
    let mut strategy_manager = state.strategy_manager;
    strategy_manager.add_strategy(&request.name)?;

    Ok(Json(()))
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct BacktestRequest {
    pub name: String,
    pub exchange: String,
    pub symbol: String,
    pub timeframe: Timeframe,
}

pub async fn backtest(
    State(state): State<AppState>,
    Json(request): Json<BacktestRequest>,
) -> ApiResult<()> {
    let backtest_context = BacktestContext {
        exchange: request.exchange,
        symbol: request.symbol,
        timeframe: request.timeframe,
    };

    let context = StrategyContext::new(state.db_pool, backtest_context)?;
    let strategy_manager = state.strategy_manager;

    strategy_manager.backtest(&request.name, context)?;
    Ok(Json(()))
}
