CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE candles (
    timestamp       TIMESTAMPTZ NOT NULL,
    exchange        TEXT NOT NULL,
    symbol          TEXT NOT NULL,
    timeframe       TEXT NOT NULL,
    open            DECIMAL(20,8) NOT NULL,
    high            DECIMAL(20,8) NOT NULL,
    low             DECIMAL(20,8) NOT NULL,
    close           DECIMAL(20,8) NOT NULL,
    volume          DECIMAL(20,8) NOT NULL,

    PRIMARY KEY (exchange, symbol, timeframe, timestamp)
);

SELECT create_hypertable('candles', 'timestamp', chunk_time_interval => INTERVAL '1 day');
