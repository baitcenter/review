use actix_web::{http, web::Data, HttpResponse};
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::{Deserialize, Serialize};

use super::schema::qualifier;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Deserialize, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "qualifier"]
#[primary_key(id)]
pub struct QualifierTable {
    pub id: i32,
    pub description: String,
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

pub(crate) fn get_qualifier_table(
    pool: Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query_result: Result<Vec<QualifierTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            qualifier::dsl::qualifier
                .load::<QualifierTable>(&conn)
                .map_err(Into::into)
        });

    let result = match query_result {
        Ok(qualifier_table) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(qualifier_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}
