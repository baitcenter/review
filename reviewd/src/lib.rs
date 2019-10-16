#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod database;
mod server;

pub use crate::database::{Example, QualifierTable, QualifierUpdate}; // for client crate
pub use crate::server::Server;
