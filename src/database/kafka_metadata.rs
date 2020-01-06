use actix_web::{
    http,
    web::{Data, Json, Query},
    HttpResponse,
};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Bound;

use super::schema::kafka_metadata;
use crate::database::{build_err_msg, lookup_kafka_metadata, Conn, Error, Pool};

#[derive(Debug, Clone, Insertable, Queryable, Serialize, Deserialize)]
#[table_name = "kafka_metadata"]
pub(crate) struct KafkaMetadata {
    pub(crate) data_source_id: i32,
    pub(crate) message_ids: (Bound<BigDecimal>, Bound<BigDecimal>),
    pub(crate) offsets: i64,
    pub(crate) partition: i32,
}

pub(crate) async fn add_kafka_metadata(
    pool: Data<Pool>,
    metadata: Json<Vec<KafkaMetadata>>,
) -> Result<HttpResponse, actix_web::Error> {
    use kafka_metadata::dsl;
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let metadata = metadata.into_inner();
        diesel::insert_into(dsl::kafka_metadata)
            .values(&metadata)
            .on_conflict((dsl::data_source_id, dsl::partition, dsl::offsets))
            .do_nothing()
            .execute(&conn)
            .map_err(Into::into)
    });
    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

pub(crate) async fn get_kafka_metadata(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    use kafka_metadata::dsl;
    let data_source_id = query
        .get("data_source_id")
        .and_then(Value::as_str)
        .and_then(|data_source_id| data_source_id.parse::<i32>().ok());

    if let Some(data_source_id) = data_source_id {
        let query_result: Result<Vec<KafkaMetadata>, Error> =
            pool.get().map_err(Into::into).and_then(|conn| {
                dsl::kafka_metadata
                    .filter(dsl::data_source_id.eq(data_source_id))
                    .select((
                        dsl::data_source_id,
                        dsl::message_ids,
                        dsl::offsets,
                        dsl::partition,
                    ))
                    .load::<KafkaMetadata>(&conn)
                    .map_err(Into::into)
            });
        match query_result {
            Ok(metadata) => Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(metadata)),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
    } else {
        Ok(HttpResponse::BadRequest().into())
    }
}

pub(crate) fn kafka_metadata_lookup(
    conn: &Conn,
    data_source_id: i32,
    message_id: &BigDecimal,
) -> Option<(u64, u64, u64)> {
    diesel::select(lookup_kafka_metadata(data_source_id, message_id))
        .get_result::<Option<Value>>(conn)
        .ok()
        .and_then(|v| v)
        .and_then(|v| {
            if let (Some(message_id), Some(offsets), Some(partition)) = (
                v.get("message_id").and_then(Value::as_u64),
                v.get("offsets").and_then(Value::as_u64),
                v.get("partition").and_then(Value::as_u64),
            ) {
                Some((message_id, partition, offsets))
            } else {
                None
            }
        })
}
