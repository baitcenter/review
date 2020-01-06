#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod app;
mod database;
mod kafka_consumer;
mod server;

use anyhow::Error;
use log::error;

pub fn log_error(e: &Error) {
    error!("{}", e);
    for cause in e.chain() {
        error!("\tcaused by: {}", cause);
    }
}
