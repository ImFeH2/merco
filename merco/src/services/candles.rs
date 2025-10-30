use crate::errors::AppResult;
use crate::models::{Candle, Timeframe};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn insert_candles(pool: &PgPool, candles: &[Candle]) -> AppResult<()> {
    if candles.is_empty() {
        return Ok(());
    }

    let mut conn = pool.acquire().await?;
    let mut copy = conn.copy_in_raw(
          "COPY candles (timestamp, exchange, symbol, timeframe, open, high, low, close, volume) FROM STDIN WITH (FORMAT
  csv)"
      ).await?;

    let mut buffer = Vec::new();
    for candle in candles {
        let line = format!(
            "{},{},{},{},{},{},{},{},{}\n",
            candle.timestamp.to_rfc3339(),
            candle.exchange,
            candle.symbol,
            candle.timeframe,
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume
        );
        buffer.extend_from_slice(line.as_bytes());
    }

    copy.send(buffer).await?;
    copy.finish().await?;

    Ok(())
}

pub async fn get_candles(
    pool: &PgPool,
    exchange: &str,
    symbol: &str,
    timeframe: Timeframe,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> AppResult<Vec<Candle>> {
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT timestamp, exchange, symbol, timeframe, open, high, low, close, volume
           FROM candles
           WHERE exchange = ",
    );

    query_builder.push_bind(exchange);
    query_builder.push(" AND symbol = ");
    query_builder.push_bind(symbol);
    query_builder.push(" AND timeframe = ");
    query_builder.push_bind(timeframe);

    if let Some(s) = start {
        query_builder.push(" AND timestamp >= ");
        query_builder.push_bind(s);
    }

    if let Some(e) = end {
        query_builder.push(" AND timestamp <= ");
        query_builder.push_bind(e);
    }

    query_builder.push(" ORDER BY timestamp ASC");

    let candles = query_builder
        .build_query_as::<Candle>()
        .fetch_all(pool)
        .await?;

    Ok(candles)
}

pub async fn get_latest_candle(
    pool: &PgPool,
    exchange: &str,
    symbol: &str,
    timeframe: Timeframe,
) -> AppResult<Option<Candle>> {
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT timestamp, exchange, symbol, timeframe, open, high, low, close, volume
           FROM candles
           WHERE exchange = ",
    );

    query_builder.push_bind(exchange);
    query_builder.push(" AND symbol = ");
    query_builder.push_bind(symbol);
    query_builder.push(" AND timeframe = ");
    query_builder.push_bind(timeframe);

    query_builder.push(" ORDER BY timestamp DESC");
    query_builder.push(" LIMIT 1");

    let latest_candle = query_builder
        .build_query_as::<Candle>()
        .fetch_optional(pool)
        .await?;

    Ok(latest_candle)
}
