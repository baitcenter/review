use crate::server;
use actix_web::dev::Server;
use anyhow::{Context, Result};

/// Creates and runs an Actix server.
///
/// # Errors
///
/// Returns an error if any of the following environment variables is invalid or
/// not set:
///
/// * `DATABASE_URL`
/// * `KAFKA_URL`
/// * `REVIEWD_ADDR`
///
/// or when it fails to run start an Actix server.
pub fn init() -> Result<Server> {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let reviewd_addr = std::env::var("REVIEWD_ADDR").context("REVIEWD_ADDR is not set")?;
    let kafka_url = std::env::var("KAFKA_URL").context("KAFKA_URL is not set")?;
    let reviewd_addr = reviewd_addr
        .parse::<std::net::SocketAddr>()
        .with_context(|| format!("invalid IP address/port for review: {}", reviewd_addr))?;

    Ok(server::run(&database_url, &reviewd_addr, kafka_url).context("failed to create server")?)
}
