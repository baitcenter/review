use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::schema::category;
use crate::database::{build_err_msg, Error, Pool};

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "category"]
pub(crate) struct CategoryTable {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewCategory {
    pub(crate) category: String,
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
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
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
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

pub(crate) fn get_category_id(pool: &Data<Pool>, category: &str) -> Result<i32, Error> {
    use category::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::category
            .select(dsl::id)
            .filter(dsl::name.eq(category))
            .first::<i32>(&conn)
            .map_err(Into::into)
    })
}

pub(crate) async fn update_category(
    pool: Data<Pool>,
    current_category: Path<String>,
    new_category: Json<NewCategory>,
) -> Result<HttpResponse, actix_web::Error> {
    use category::dsl;

    let current_category = current_category.into_inner();
    let update_result = get_category_id(&pool, &current_category).and_then(|_| {
        let target = dsl::category.filter(dsl::name.eq(current_category));
        pool.get().map_err(Into::into).and_then(|conn| {
            diesel::update(target)
                .set(dsl::name.eq(new_category.into_inner().category))
                .execute(&conn)
                .map_err(Into::into)
        })
    });

    match update_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
