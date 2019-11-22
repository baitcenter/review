use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde_json::Value;

use super::schema::indicator;
use crate::database::*;

pub(crate) fn add_indicator(
    pool: Data<Pool>,
    indicators: Json<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use indicator::dsl;

    let name = indicators.get("name").and_then(Value::as_str);
    let token: Option<&Value> = indicators.get("token");
    let description = indicators.get("description").and_then(Value::as_str);
    let data_source_id = indicators
        .get("data_source")
        .and_then(Value::as_str)
        .map(|d| get_data_source_id(&pool, d));

    let result = if let (Some(name), Some(token), Some(Ok(data_source_id))) =
        (name, token, data_source_id)
    {
        let insert_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
            diesel::insert_into(dsl::indicator)
                .values((
                    dsl::name.eq(name),
                    dsl::token.eq(token),
                    dsl::description.eq(description),
                    dsl::data_source_id.eq(data_source_id),
                ))
                .execute(&conn)
                .map_err(Into::into)
        });

        match insert_result {
            Ok(_) => Ok(HttpResponse::Created().into()),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
    } else {
        Ok(HttpResponse::BadRequest().into())
    };

    future::result(result)
}

pub(crate) fn delete_indicator(
    pool: Data<Pool>,
    query: Query<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use indicator::dsl;
    let is_all = query
        .get("all")
        .and_then(Value::as_str)
        .and_then(|all| match all.to_lowercase().as_str() {
            "true" => Some(true),
            _ => None,
        })
        .unwrap_or_else(|| false);
    let name = query.get("name").and_then(Value::as_str);

    if let (false, None) | (true, Some(_)) = (is_all, name) {
        return future::result(Ok(HttpResponse::BadRequest().into()));
    }

    let delete_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        if let Some(name) = name {
            diesel::delete(dsl::indicator.filter(dsl::name.eq(name)))
                .execute(&conn)
                .map_err(Into::into)
        } else {
            diesel::delete(dsl::indicator)
                .execute(&conn)
                .map_err(Into::into)
        }
    });

    let result = match delete_result {
        Ok(0) => Ok(HttpResponse::BadRequest().into()),
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };
    future::result(result)
}

pub(crate) fn get_indicators(
    pool: Data<Pool>,
    query: Query<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let indicator_schema =
        "(indicator INNER JOIN data_source ON indicator.data_source_id = data_source.id)";
    let select = vec![
        "indicator.name",
        "indicator.token",
        "data_source.topic_name as data_source",
        "indicator.description",
        "indicator.last_modification_time",
    ];
    let filter = query
        .get("filter")
        .and_then(Value::as_str)
        .and_then(|f| serde_json::from_str::<Value>(f).ok());
    let where_clause = if let Some(filter) = filter {
        filter.get("name").and_then(Value::as_array).map(|f| {
            let mut where_clause = String::new();
            for (index, f) in f.iter().enumerate() {
                if let Some(f) = f.as_str() {
                    let filter = format!("indicator.name = '{}'", f);
                    if index == 0 {
                        where_clause.push_str(&filter);
                    } else {
                        where_clause.push_str(&format!(" or {}", filter));
                    }
                }
            }
            where_clause
        })
    } else {
        None
    };
    let default_per_page = 10;
    let max_per_page = 100;
    let page = GetQuery::get_page(&query);
    let per_page = GetQuery::get_per_page(&query, max_per_page).unwrap_or_else(|| default_per_page);
    let orderby = query
        .get("orderby")
        .and_then(Value::as_str)
        .and_then(|column_name| match column_name.to_lowercase().as_str() {
            "name" => Some("indicator.name"),
            "token" => Some("indicator.token"),
            _ => None,
        });
    let order = if orderby.is_some() {
        GetQuery::get_order(&query)
    } else {
        None
    };

    let query_result: Result<Vec<GetQueryData>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            GetQuery::new(
                select,
                indicator_schema,
                where_clause,
                page,
                per_page,
                orderby,
                order,
            )
            .get_results::<GetQueryData>(&conn)
            .map_err(Into::into)
        });

    future::result(GetQuery::build_response(&query, per_page, query_result))
}

pub(crate) fn update_indicator(
    pool: Data<Pool>,
    name: Path<String>,
    new_indicator: Json<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let new_indicator = new_indicator.into_inner();
    let (new_name, new_data_source, new_description, new_token) = (
        new_indicator.get("name").and_then(Value::as_str),
        new_indicator.get("data_source").and_then(Value::as_str),
        new_indicator.get("description").and_then(Value::as_str),
        new_indicator.get("token"),
    );

    if let (Some(_), _, _, _) | (_, Some(_), _, _) | (_, _, Some(_), _) | (_, _, _, Some(_)) =
        (new_name, new_data_source, new_description, new_token)
    {
        let query_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
            let name = name.into_inner();
            diesel::select(attempt_indicator_update(
                name,
                new_name,
                new_token,
                new_data_source,
                new_description,
            ))
            .get_result::<i32>(&conn)
            .map_err(Into::into)
        });
        let result = match query_result {
            Ok(1) => Ok(HttpResponse::Ok().into()),
            Ok(_) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(json!({"message": "Something went wrong"}).to_string())),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        };
        future::result(result)
    } else {
        future::result(Ok(HttpResponse::BadRequest().into()))
    }
}
