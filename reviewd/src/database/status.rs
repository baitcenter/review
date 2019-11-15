use actix_web::{http, web::Data, HttpResponse};
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::Serialize;

use super::schema::status;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Identifiable, Queryable, Serialize)]
#[table_name = "status"]
pub(crate) struct StatusTable {
    pub(crate) id: i32,
    pub(crate) description: String,
}

pub(crate) fn get_status_id(pool: &Data<Pool>, status: &str) -> Result<i32, Error> {
    use status::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::status
            .select(dsl::id)
            .filter(dsl::description.eq(status))
            .first::<i32>(&conn)
            .map_err(Into::into)
    })
}

pub(crate) fn get_status_table(
    pool: Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query_result: Result<Vec<StatusTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            status::dsl::status
                .load::<StatusTable>(&conn)
                .map_err(Into::into)
        });

    let result = match query_result {
        Ok(status_table) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(status_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}
