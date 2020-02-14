use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::schema::category;
use crate::database::{build_http_500_response, Error, Pool};

#[derive(Debug, Identifiable, Insertable, Queryable, Serialize)]
#[table_name = "category"]
struct CategoryTable {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewCategory {
    category: String,
}

pub(crate) async fn add_category(
    pool: Data<Pool>,
    new_category: Query<NewCategory>,
) -> Result<HttpResponse, actix_web::Error> {
    use category::dsl;
    let insert_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        diesel::insert_into(dsl::category)
            .values(dsl::name.eq(&new_category.into_inner().category))
            .execute(&conn)
            .map_err(Into::into)
    });

    match insert_result {
        Ok(category) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(category)),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}

pub(crate) async fn get_category_table(pool: Data<Pool>) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<Vec<CategoryTable>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            category::dsl::category
                .load::<CategoryTable>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(category) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(category)),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}

pub(crate) async fn update_category(
    pool: Data<Pool>,
    current_category: Path<String>,
    new_category: Json<NewCategory>,
) -> Result<HttpResponse, actix_web::Error> {
    use category::dsl;
    let update_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        diesel::update(dsl::category)
            .filter(dsl::name.eq(&current_category.into_inner()))
            .set(dsl::name.eq(new_category.into_inner().category))
            .execute(&conn)
            .map_err(Into::into)
    });

    match update_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}
