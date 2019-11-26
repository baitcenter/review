#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod app;
mod database;
mod server;

use anyhow::Error;
use log::error;
use server::Server;

pub fn log_error(e: &Error) {
    error!("{}", e);
    for cause in e.chain() {
        error!("\tcaused by: {}", cause);
    }
}
