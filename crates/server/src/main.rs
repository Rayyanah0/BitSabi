//! BitSabi API entry point.

use std::error::Error;

use bitsabi_server::{app, config::Config};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("bitsabi_server=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = Config::from_env()?;
    let listener = TcpListener::bind(config.bind_addr).await?;

    tracing::info!(
        address = %config.bind_addr,
        environment = config.environment.as_str(),
        "BitSabi API listening"
    );

    axum::serve(listener, app(&config))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to listen for shutdown signal");
    }
}
