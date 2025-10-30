use crate::errors::{AppError, AppResult};
use crate::models::{Candle, Timeframe};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{TimeZone, Utc};
use pyo3::{prelude::*, types::PyDict};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug)]
pub struct CCXT {
    exchange_name: String,
    instance: Py<PyAny>,
}

impl CCXT {
    const MODULE_NAME: &str = "ccxt";

    pub fn exchanges() -> AppResult<Vec<String>> {
        Python::attach(|py| {
            let ccxt = py.import(CCXT::MODULE_NAME)?;
            Ok(ccxt.getattr("exchanges")?.extract()?)
        })
    }

    pub fn try_from_exchange(exchange: &str) -> AppResult<CCXT> {
        Python::attach(|py| {
            let ccxt = py.import(CCXT::MODULE_NAME)?;
            let exchange_class = ccxt.getattr(exchange)?;
            let exchange_instance = exchange_class.call0()?;
            exchange_instance.call_method0("load_markets")?;

            Ok(CCXT {
                exchange_name: exchange.to_string(),
                instance: exchange_instance.unbind(),
            })
        })
    }

    pub fn symbols(&self) -> AppResult<Vec<String>> {
        Python::attach(|py| {
            let exchange = self.instance.bind(py);
            Ok(exchange.getattr("symbols")?.extract()?)
        })
    }

    pub fn timeframes(&self) -> AppResult<HashMap<Timeframe, String>> {
        Python::attach(|py| {
            let exchange = self.instance.bind(py);
            let timeframes_any = exchange.getattr("timeframes")?;
            let timeframes_dict = timeframes_any.cast::<PyDict>()?;

            let mut timeframes = HashMap::new();
            for (k, v) in timeframes_dict.iter() {
                let key: String = k.extract()?;
                let val: String = v.extract()?;
                timeframes.insert(Timeframe::from_str(&key)?, val);
            }

            Ok(timeframes)
        })
    }

    pub fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<u64>,
        limit: Option<u64>,
    ) -> AppResult<Vec<Candle>> {
        fn f64_to_bigdecimal(value: f64, field_name: &str) -> AppResult<BigDecimal> {
            BigDecimal::from_f64(value)
                .ok_or_else(|| format!("Invalid {}: {}", field_name, value).into())
        }

        Python::attach(|py| {
            let exchange = self.instance.bind(py);
            let args = (symbol, timeframe.to_string(), since, limit);

            let candles_any = exchange.call_method("fetch_ohlcv", args, None)?;
            let candles_data: Vec<[f64; 6]> = candles_any.extract()?;

            let mut candles = Vec::new();
            for [timestamp_ms, open, high, low, close, volume] in candles_data {
                let Some(timestamp) = Utc.timestamp_millis_opt(timestamp_ms as i64).single() else {
                    return Err(format!("Error while parse timestamp: {}", timestamp_ms).into());
                };

                candles.push(Candle {
                    timestamp,
                    exchange: self.exchange_name.clone(),
                    symbol: symbol.to_string(),
                    timeframe,
                    open: f64_to_bigdecimal(open, "open price")?,
                    high: f64_to_bigdecimal(high, "high price")?,
                    low: f64_to_bigdecimal(low, "low price")?,
                    close: f64_to_bigdecimal(close, "close price")?,
                    volume: f64_to_bigdecimal(volume, "volume")?,
                });
            }

            Ok(candles)
        })
    }
}
