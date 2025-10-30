mod app;
mod errors;
mod exchange;
pub mod handlers;
pub mod models;
pub mod services;
pub mod strategy;
pub mod tasks;
mod utils;

pub use crate::errors::AppResult;
pub use crate::models::{Candle, Timeframe};
pub use crate::strategy::{Strategy, StrategyContext};
pub use strategy_macro::strategy;
