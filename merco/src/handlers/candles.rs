use crate::app::AppState;
use crate::errors::ApiResult;
use crate::models::{Candle, Timeframe};
use crate::services;
use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc, serde::ts_milliseconds_option};
use serde::Deserialize;
use ts_rs::TS;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct GetCandlesQuery {
    pub symbol: String,
    pub exchange: String,
    pub timeframe: Timeframe,
    #[serde(default, with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub start: Option<DateTime<Utc>>,
    #[serde(default, with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub end: Option<DateTime<Utc>>,
}

pub async fn get_candles(
    State(state): State<AppState>,
    Query(query): Query<GetCandlesQuery>,
) -> ApiResult<Vec<Candle>> {
    let candles = services::candles::get_candles(
        &state.db_pool,
        &query.exchange,
        &query.symbol,
        query.timeframe,
        query.start,
        query.end,
    )
    .await?;

    Ok(Json(candles))
}
