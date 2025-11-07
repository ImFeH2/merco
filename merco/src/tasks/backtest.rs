use crate::errors::AppResult;
use crate::exchange::ccxt::CCXT;
use crate::models::Timeframe;
use crate::services;
use crate::strategy::{StrategyContext, StrategyHandle, Trade};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc, serde::ts_milliseconds, serde::ts_milliseconds_option};
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::broadcast;
use ts_rs::TS;
use uuid::Uuid;

const BACKTEST_BROADCAST_INTERVAL: usize = 100;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BacktestResult {
    pub exchange: String,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub candles_processed: usize,
    #[ts(type = "string")]
    pub final_balance: BigDecimal,
    #[ts(type = "string")]
    pub final_position: BigDecimal,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BacktestStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BacktestTask {
    pub id: Uuid,
    pub status: BacktestStatus,
    pub progress: f32,
    pub name: String,
    pub exchange: String,
    pub symbol: String,
    pub timeframe: Timeframe,
    #[ts(optional)]
    pub result: Option<BacktestResult>,
    #[ts(optional)]
    pub error_message: Option<String>,
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    #[ts(skip)]
    pub event_tx: broadcast::Sender<BacktestTask>,
}

impl BacktestTask {
    pub fn broadcast(&self) {
        let _ = self.event_tx.send(self.clone());
    }

    pub async fn execute(&mut self, db_pool: PgPool, strategy_handle: StrategyHandle) {
        let now = Utc::now();
        self.status = BacktestStatus::Running;
        self.started_at = Some(now);
        self.updated_at = now;
        self.broadcast();

        let result = self.execute_backtest(db_pool, strategy_handle).await;
        let now = Utc::now();
        match result {
            Ok(backtest_result) => {
                self.status = BacktestStatus::Completed;
                self.progress = 100.0;
                self.result = Some(backtest_result);
                self.completed_at = Some(now);
                self.updated_at = now;
            }
            Err(e) => {
                self.status = BacktestStatus::Failed;
                self.error_message = Some(e.to_string());
                self.completed_at = Some(now);
                self.updated_at = now;
            }
        };

        self.broadcast();
    }

    async fn execute_backtest(
        &mut self,
        db_pool: PgPool,
        mut strategy_handle: StrategyHandle,
    ) -> AppResult<BacktestResult> {
        let exchange = self.exchange.clone();
        let symbol = self.symbol.clone();
        let timeframe = self.timeframe;

        tracing::info!(
            "Running backtest on {}/{} with timeframe {}",
            exchange,
            symbol,
            timeframe
        );

        let all_candles =
            services::candles::get_candles(&db_pool, &exchange, &symbol, timeframe, None, None)
                .await?;

        let total_candles = all_candles.len();
        if total_candles == 0 {
            return Err("No candles available for backtest".into());
        }

        let ccxt = CCXT::with_exchange(&exchange)?;
        let fees = ccxt.fees(&symbol)?;
        let precision = ccxt.precision(&symbol)?;
        let mut context = StrategyContext::new(fees, precision)?;

        for (index, candle) in all_candles.into_iter().enumerate() {
            context.candles.push(candle);

            context.before()?;
            strategy_handle.tick(&mut context)?;
            context.after()?;

            if index % BACKTEST_BROADCAST_INTERVAL == 0 {
                let progress = 100.0 * ((index + 1) as f32) / (total_candles as f32);
                self.progress = progress;
                self.updated_at = Utc::now();
                self.broadcast();
            }
        }

        context.end()?;
        self.progress = 100.0;
        self.updated_at = Utc::now();
        self.broadcast();

        Ok(BacktestResult {
            exchange,
            symbol,
            timeframe,
            candles_processed: total_candles,
            final_balance: context.balance,
            final_position: context.position,
            trades: context.trades,
        })
    }
}
