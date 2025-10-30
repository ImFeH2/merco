use crate::{
    errors::{AppError, AppResult},
    models::Candle,
    models::Timeframe,
    services::candles::get_latest_candle,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct BacktestContext {
    pub exchange: String,
    pub symbol: String,
    pub timeframe: Timeframe,
}

#[derive(Debug, Clone)]
pub struct StrategyContext {
    db_pool: PgPool,
    backtest_ctx: BacktestContext,
}

impl StrategyContext {
    pub(crate) fn new(db_pool: PgPool, backtest_ctx: BacktestContext) -> AppResult<Self> {
        Ok(Self {
            db_pool,
            backtest_ctx,
        })
    }

    pub fn candle(&self) -> AppResult<Candle> {
        let runtime = tokio::runtime::Runtime::new()?;
        let _guard = runtime.enter();

        let candle = runtime
            .block_on(async {
                get_latest_candle(
                    &self.db_pool,
                    &self.backtest_ctx.exchange,
                    &self.backtest_ctx.symbol,
                    self.backtest_ctx.timeframe,
                )
                .await
            })?
            .ok_or_else(|| "No candle found")?;

        Ok(candle)
    }

    pub fn long(&self) -> AppResult<()> {
        println!("call long!");
        Ok(())
    }
}
