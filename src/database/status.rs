use actix_web::{http, web::Data, HttpResponse};
use diesel::prelude::*;
use serde::Serialize;

use super::schema::status;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Identifiable, Queryable, Serialize)]
#[table_name = "status"]
struct StatusTable {
    id: i32,
    description: String,
}

pub(crate) async fn get_status_table(pool: Data<Pool>) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<Vec<StatusTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            status::dsl::status
                .load::<StatusTable>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(status_table) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(status_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
