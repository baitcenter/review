use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::PooledConnection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schema::data_source;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Deserialize)]
pub(crate) struct DataSourceQuery {
    pub(crate) data_source: String,
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
#[table_name = "data_source"]
struct DataSourceTable {
    id: i32,
    topic_name: String,
    data_type: String,
}

pub(crate) async fn add_data_source_endpoint(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let data_source = query.get("data_source").and_then(Value::as_str);
    let data_type = query.get("data_type").and_then(Value::as_str);

    let query_result = if let (Some(data_source), Some(data_type)) = (data_source, data_type) {
        add_data_source(&pool, data_source, data_type)
    } else {
        return Ok(HttpResponse::BadRequest().into());
    };

    match query_result {
        0 => Ok(HttpResponse::InternalServerError().into()),
        _ => Ok(HttpResponse::Created().into()),
    }
}

pub(crate) fn add_data_source(pool: &Data<Pool>, data_source: &str, data_type: &str) -> i32 {
    use data_source::dsl;

    let new_data_source: Result<DataSourceTable, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            diesel::insert_into(dsl::data_source)
                .values((
                    dsl::topic_name.eq(data_source),
                    dsl::data_type.eq(data_type),
                ))
                .on_conflict(dsl::topic_name)
                .do_nothing()
                .get_result(&conn)
                .map_err(Into::into)
        });

    new_data_source.ok().map_or(0, |d| d.id)
}

pub(crate) fn get_data_source_id(pool: &Data<Pool>, data_source: &str) -> Result<i32, Error> {
    pool.get()
        .map_err(Into::into)
        .and_then(|conn| id(&conn, data_source))
}

pub(crate) fn id(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    data_source: &str,
) -> Result<i32, Error> {
    use data_source::dsl;
    dsl::data_source
        .select(dsl::id)
        .filter(dsl::topic_name.eq(data_source))
        .first::<i32>(conn)
        .map_err(Into::into)
}

pub(crate) async fn get_data_source_table(
    pool: Data<Pool>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<Vec<DataSourceTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            data_source::dsl::data_source
                .load::<DataSourceTable>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(data_source_table) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(data_source_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
