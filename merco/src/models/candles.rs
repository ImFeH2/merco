use bigdecimal::BigDecimal;
use chrono::{DateTime, TimeDelta, Utc, serde::ts_milliseconds};
use core::fmt;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::{hash::Hash, str::FromStr};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Candle {
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    pub timeframe: Timeframe,
    #[ts(type = "string")]
    pub open: BigDecimal,
    #[ts(type = "string")]
    pub high: BigDecimal,
    #[ts(type = "string")]
    pub low: BigDecimal,
    #[ts(type = "string")]
    pub close: BigDecimal,
    #[ts(type = "string")]
    pub volume: BigDecimal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Type, TS)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[ts(export)]
pub enum Timeframe {
    #[serde(rename = "1s")]
    #[sqlx(rename = "1s")]
    S1,

    #[serde(rename = "1m")]
    #[sqlx(rename = "1m")]
    M1,

    #[serde(rename = "3m")]
    #[sqlx(rename = "3m")]
    M3,

    #[serde(rename = "5m")]
    #[sqlx(rename = "5m")]
    M5,

    #[serde(rename = "15m")]
    #[sqlx(rename = "15m")]
    M15,

    #[serde(rename = "30m")]
    #[sqlx(rename = "30m")]
    M30,

    #[serde(rename = "1h")]
    #[sqlx(rename = "1h")]
    H1,

    #[serde(rename = "2h")]
    #[sqlx(rename = "2h")]
    H2,

    #[serde(rename = "4h")]
    #[sqlx(rename = "4h")]
    H4,

    #[serde(rename = "6h")]
    #[sqlx(rename = "6h")]
    H6,

    #[serde(rename = "8h")]
    #[sqlx(rename = "8h")]
    H8,

    #[serde(rename = "12h")]
    #[sqlx(rename = "12h")]
    H12,

    #[serde(rename = "1d")]
    #[sqlx(rename = "1d")]
    D1,

    #[serde(rename = "3d")]
    #[sqlx(rename = "3d")]
    D3,

    #[serde(rename = "1w")]
    #[sqlx(rename = "1w")]
    W1,

    #[serde(rename = "1M")]
    #[sqlx(rename = "1M")]
    MN1,
}

impl Timeframe {
    pub fn to_ms(&self) -> u64 {
        self.to_delta().num_milliseconds() as u64
    }
    pub fn to_delta(&self) -> TimeDelta {
        match self {
            Timeframe::S1 => TimeDelta::seconds(1),
            Timeframe::M1 => TimeDelta::minutes(1),
            Timeframe::M3 => TimeDelta::minutes(3),
            Timeframe::M5 => TimeDelta::minutes(5),
            Timeframe::M15 => TimeDelta::minutes(15),
            Timeframe::M30 => TimeDelta::minutes(30),
            Timeframe::H1 => TimeDelta::hours(1),
            Timeframe::H2 => TimeDelta::hours(2),
            Timeframe::H4 => TimeDelta::hours(4),
            Timeframe::H6 => TimeDelta::hours(6),
            Timeframe::H8 => TimeDelta::hours(8),
            Timeframe::H12 => TimeDelta::hours(12),
            Timeframe::D1 => TimeDelta::days(1),
            Timeframe::D3 => TimeDelta::days(3),
            Timeframe::W1 => TimeDelta::weeks(1),
            Timeframe::MN1 => TimeDelta::days(30),
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json.trim_matches('"'))
    }
}

impl FromStr for Timeframe {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{}\"", s))
    }
}
