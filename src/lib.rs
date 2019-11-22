#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod app;
mod database;
mod error;
mod server;

use failure::Fail;
use log::error;
use server::Server;

pub fn log_error(fail: &dyn Fail) {
    error!("{}", fail);
    for cause in fail.iter_causes() {
        error!("\tcaused by: {}", cause);
    }
}
