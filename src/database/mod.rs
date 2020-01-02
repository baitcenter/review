use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use log::error;
use serde_json::json;
use thiserror::Error;

mod category;
mod cluster;
mod data_source;
mod description;
mod event;
mod function;
mod indicator;
mod kafka_metadata;
mod outlier;
mod qualifier;
mod query;
mod raw_event;
mod schema;
mod status;

pub(crate) use self::category::*;
pub(crate) use self::cluster::*;
pub(crate) use self::data_source::*;
pub(crate) use self::description::*;
pub(crate) use self::event::*;
pub(crate) use self::function::*;
pub(crate) use self::indicator::*;
pub(crate) use self::kafka_metadata::*;
pub(crate) use self::outlier::*;
pub(crate) use self::qualifier::*;
pub(crate) use self::query::*;
pub(crate) use self::raw_event::*;
pub(crate) use self::status::*;

pub(crate) type Conn = PooledConnection<ConnectionManager<PgConnection>>;
pub(crate) type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("record does not exist")]
    RecordNotExist,
    #[error("transaction error")]
    Transaction,
    #[error("diesel connection error: {0}")]
    Connection(#[from] diesel::ConnectionError),
    #[error("migration error: {0}")]
    Migration(#[from] diesel_migrations::RunMigrationsError),
    #[error("query error: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("connection error: {0}")]
    R2D2(#[from] r2d2::Error),
    #[error("JSON deserialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

pub(crate) fn build_err_msg(e: &dyn std::error::Error) -> String {
    error!("{}", e);
    json!({
        "message": e.to_string(),
    })
    .to_string()
}

pub(crate) fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}
