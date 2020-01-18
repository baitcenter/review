use crate::server;
use actix_web::dev::Server;
use anyhow::{Context, Result};
use clap::{App, Arg};

fn create_app() -> App<'static, 'static> {
    App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(Arg::with_name("subcommand")) // TODO: Remove in 0.8.0.
}

pub fn init() -> Result<Server> {
    let matches = create_app().get_matches();
    if matches.value_of("subcommand").is_some() {
        eprintln!("Warning: A subcommand is deprecated. Please run review without subcommand.");
    }
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let reviewd_addr = std::env::var("REVIEWD_ADDR").context("REVIEWD_ADDR is not set")?;
    let kafka_url = std::env::var("KAFKA_URL").context("KAFKA_URL is not set")?;
    let reviewd_addr = reviewd_addr
        .parse::<std::net::SocketAddr>()
        .with_context(|| format!("invalid IP address/port for review: {}", reviewd_addr))?;

    Ok(server::run(&database_url, &reviewd_addr, kafka_url).context("failed to create server")?)
}
