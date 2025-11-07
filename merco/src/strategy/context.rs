use crate::errors::{AppError, AppResult};
use crate::models::{Candle, MarketPrecision, TradingFees};
use crate::utils::{round_down_to_precision, round_up_to_precision};
use bigdecimal::{BigDecimal, Zero};
use chrono::{DateTime, Utc, serde::ts_milliseconds};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TradeType {
    MarketBuy,
    MarketSell,
    LimitBuy,
    LimitSell,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Trade {
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub timestamp: DateTime<Utc>,
    pub trade_type: TradeType,
    #[ts(type = "string")]
    pub price: BigDecimal,
    #[ts(type = "string")]
    pub amount: BigDecimal,
    #[ts(type = "string")]
    pub fee: BigDecimal,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrderType {
    LimitBuy,
    LimitSell,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: Uuid,
    pub order_type: OrderType,
    pub price: BigDecimal,
    pub amount: BigDecimal,
    pub fee: BigDecimal,
}

#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub(crate) candles: Vec<Candle>,
    pub(crate) balance: BigDecimal,
    pub(crate) position: BigDecimal,
    pub(crate) trades: Vec<Trade>,
    pub(crate) orders: Vec<Order>,
    pub(crate) fees: TradingFees,
    pub(crate) precision: MarketPrecision,
}

impl StrategyContext {
    pub(crate) fn new(fees: TradingFees, precision: MarketPrecision) -> AppResult<Self> {
        Ok(Self {
            candles: Vec::new(),
            balance: BigDecimal::from(10000),
            position: BigDecimal::zero(),
            trades: Vec::new(),
            orders: Vec::new(),
            fees,
            precision,
        })
    }

    pub(crate) fn before(&mut self) -> AppResult<()> {
        let candle = self
            .candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))?;

        let mut executed_orders = Vec::new();
        for order in &self.orders {
            match order.order_type {
                OrderType::LimitBuy => {
                    if order.price >= candle.low {
                        self.position += &order.amount;
                        self.trades.push(Trade {
                            timestamp: candle.timestamp,
                            trade_type: TradeType::LimitBuy,
                            price: order.price.clone(),
                            amount: order.amount.clone(),
                            fee: order.fee.clone(),
                        });
                        executed_orders.push(order.id);
                    }
                }
                OrderType::LimitSell => {
                    if order.price <= candle.high {
                        let proceeds = &order.price * &order.amount;
                        let fee = &order.fee;
                        self.balance += &proceeds - fee;
                        self.trades.push(Trade {
                            timestamp: candle.timestamp,
                            trade_type: TradeType::LimitSell,
                            price: order.price.clone(),
                            amount: order.amount.clone(),
                            fee: order.fee.clone(),
                        });
                        executed_orders.push(order.id);
                    }
                }
            }
        }

        self.orders
            .retain(|order| !executed_orders.contains(&order.id));

        Ok(())
    }

    pub(crate) fn after(&mut self) -> AppResult<()> {
        Ok(())
    }

    pub(crate) fn end(&mut self) -> AppResult<()> {
        let order_ids: Vec<Uuid> = self.orders.iter().map(|o| o.id).collect();
        for id in order_ids {
            self.cancel_order(id);
        }

        Ok(())
    }

    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    pub fn candle(&self) -> AppResult<&Candle> {
        self.candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))
    }

    pub fn balance(&self) -> BigDecimal {
        self.balance.clone()
    }

    pub fn position(&self) -> BigDecimal {
        self.position.clone()
    }

    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    pub fn precision(&self) -> &MarketPrecision {
        &self.precision
    }

    pub fn cancel_order(&mut self, order_id: Uuid) {
        if let Some(pos) = self.orders.iter().position(|o| o.id == order_id) {
            let order = &self.orders[pos];
            match order.order_type {
                OrderType::LimitBuy => {
                    let refund = &order.price * &order.amount + &order.fee;
                    self.balance += &refund;
                }
                OrderType::LimitSell => {
                    self.position += &order.amount;
                }
            }
            self.orders.remove(pos);
        }
    }

    pub fn market_buy(&mut self, amount: &BigDecimal) -> AppResult<()> {
        let amount = round_down_to_precision(amount, &self.precision.amount_precision);
        if amount <= BigDecimal::zero() {
            return Err(AppError::Strategy("Amount must be positive".into()));
        }

        let candle = self
            .candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))?;
        let price = round_down_to_precision(&candle.close, &self.precision.price_precision);
        let timestamp = candle.timestamp;

        let cost = &price * &amount;
        let fee =
            round_up_to_precision(&(&cost * &self.fees.taker), &self.precision.price_precision);
        let total = &cost + &fee;

        if total > self.balance {
            return Err(AppError::Strategy("Insufficient funds".into()));
        }

        self.balance -= &total;
        self.position += &amount;

        self.trades.push(Trade {
            timestamp,
            trade_type: TradeType::MarketBuy,
            price,
            amount,
            fee,
        });

        Ok(())
    }

    pub fn market_sell(&mut self, amount: &BigDecimal) -> AppResult<()> {
        let amount = round_down_to_precision(amount, &self.precision.amount_precision);
        if amount <= BigDecimal::zero() {
            return Err(AppError::Strategy("Amount must be positive".into()));
        }
        if amount > self.position {
            return Err(AppError::Strategy(
                "Insufficient base asset amount to sell".into(),
            ));
        }

        let candle = self
            .candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))?;
        let price = round_down_to_precision(&candle.close, &self.precision.price_precision);
        let timestamp = candle.timestamp;

        let proceeds = &price * &amount;
        let fee = round_up_to_precision(
            &(&proceeds * &self.fees.taker),
            &self.precision.price_precision,
        );

        self.position -= &amount;
        self.balance += &proceeds - &fee;

        self.trades.push(Trade {
            timestamp,
            trade_type: TradeType::MarketSell,
            price,
            amount,
            fee,
        });

        Ok(())
    }

    pub fn limit_buy(
        &mut self,
        price: &BigDecimal,
        amount: &BigDecimal,
    ) -> AppResult<Option<Uuid>> {
        let price = round_down_to_precision(price, &self.precision.price_precision);
        let amount = round_down_to_precision(amount, &self.precision.amount_precision);

        if amount <= BigDecimal::zero() {
            return Err(AppError::Strategy("Amount must be positive".into()));
        }

        let candle = self
            .candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))?;

        if price >= candle.close {
            self.market_buy(&amount)?;
            return Ok(None);
        };

        let cost = &amount * &price;
        let fee =
            round_up_to_precision(&(&cost * &self.fees.maker), &self.precision.price_precision);

        let total = &cost + &fee;
        if total > self.balance {
            return Err(AppError::Strategy("Insufficient funds".into()));
        }

        self.balance -= &total;

        let order_id = Uuid::new_v4();
        self.orders.push(Order {
            id: order_id,
            order_type: OrderType::LimitBuy,
            price,
            amount,
            fee,
        });

        Ok(Some(order_id))
    }

    pub fn limit_sell(
        &mut self,
        price: &BigDecimal,
        amount: &BigDecimal,
    ) -> AppResult<Option<Uuid>> {
        let price = round_down_to_precision(price, &self.precision.price_precision);
        let amount = round_down_to_precision(amount, &self.precision.amount_precision);

        if amount <= BigDecimal::zero() {
            return Err(AppError::Strategy("Amount must be positive".into()));
        }

        if amount > self.position {
            return Err(AppError::Strategy(
                "Insufficient base asset amount to sell".into(),
            ));
        }

        let candle = self
            .candles
            .last()
            .ok_or(AppError::Strategy("No candles available".into()))?;

        if price <= candle.close {
            self.market_sell(&amount)?;
            return Ok(None);
        };

        let proceeds = &price * &amount;
        let fee = round_up_to_precision(
            &(&proceeds * &self.fees.maker),
            &self.precision.price_precision,
        );
        self.position -= &amount;

        let order_id = Uuid::new_v4();
        self.orders.push(Order {
            id: order_id,
            order_type: OrderType::LimitSell,
            price,
            amount,
            fee,
        });

        Ok(Some(order_id))
    }
}
