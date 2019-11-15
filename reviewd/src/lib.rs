#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod database;
mod server;

pub use crate::server::Server;
