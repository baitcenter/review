use actix_web::{web::{BytesMut, Payload}, http, HttpResponse};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use futures::StreamExt;
use log::error;
use serde_json::json;
use thiserror::Error;

mod category;
mod cluster;
mod data_source;
mod description;
mod event;
mod event_id;
mod function;
mod indicator;
mod kafka_metadata;
mod outlier;
mod qualifier;
mod query;
mod schema;
mod status;
mod template;

pub(crate) use self::category::*;
pub(crate) use self::cluster::*;
pub(crate) use self::data_source::*;
pub(crate) use self::description::*;
pub(crate) use self::event::*;
pub(crate) use self::event_id::*;
pub(crate) use self::function::*;
pub(crate) use self::indicator::*;
pub(crate) use self::kafka_metadata::*;
pub(crate) use self::outlier::*;
pub(crate) use self::qualifier::*;
pub(crate) use self::query::*;
pub(crate) use self::status::*;
pub(crate) use self::template::*;

pub(crate) type Conn = PooledConnection<ConnectionManager<PgConnection>>;
pub(crate) type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Error)]
pub enum Error {
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

fn build_err_msg(e: &dyn std::error::Error) -> String {
    error!("{}", e);
    json!({
        "message": e.to_string(),
    })
    .to_string()
}

pub(crate) fn build_http_500_response(e: &dyn std::error::Error) -> HttpResponse {
    HttpResponse::InternalServerError()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(build_err_msg(e))
}

pub(crate) async fn load_payload(mut payload: Payload) -> Result<BytesMut, actix_web::Error> {
    let mut bytes = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        bytes.extend_from_slice(&chunk?);
    }
    Ok(bytes)
}
