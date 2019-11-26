use actix_web::{http, web::Data, HttpResponse};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::schema::qualifier;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[table_name = "qualifier"]
pub(crate) struct QualifierTable {
    pub(crate) id: i32,
    pub(crate) description: String,
}

pub(crate) fn get_qualifier_id(pool: &Data<Pool>, qualifier: &str) -> Result<i32, Error> {
    use qualifier::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::qualifier
            .select(dsl::id)
            .filter(dsl::description.eq(qualifier))
            .first::<i32>(&conn)
            .map_err(Into::into)
    })
}

pub(crate) async fn get_qualifier_table(
    pool: Data<Pool>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<Vec<QualifierTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            qualifier::dsl::qualifier
                .load::<QualifierTable>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(qualifier_table) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(qualifier_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
