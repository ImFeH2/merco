mod app;
mod config;
mod errors;
mod exchange;
mod handlers;
mod models;
mod services;
mod strategy;
mod tasks;
mod utils;

use sqlx::postgres::PgPoolOptions;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = config::Config::load().expect("Failed to load config");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(config.log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();
    tracing::info!("Loaded configuration");

    tracing::info!("Connecting to database at {}", config.database.url);
    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("Connected to database");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    let token = CancellationToken::new();
    let app = app::create_app(db_pool, token.clone());

    let host = Ipv4Addr::from_str(&config.server.host).expect("Invalid server host IP");
    let addr = SocketAddrV4::new(host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Server listening on {}", addr);

    async fn shutdown_signal(token: CancellationToken) {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c signal");
        tracing::info!("Ctrl+C received, shutting down...");
        token.cancel();
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token.clone()))
        .await
        .unwrap();
}
