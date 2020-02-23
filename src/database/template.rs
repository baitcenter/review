use actix_web::{
    http,
    web::{Data, Payload, Query},
    HttpResponse,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schema::template;
use crate::database::{build_http_500_response, load_payload, Error, Pool};

#[derive(Debug, Insertable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "template"]
struct Template {
    name: String,
    event_type: String,
    method: String,
    algorithm: Option<String>,
    min_token_length: Option<i64>,
    eps: Option<f64>,
    format: Option<Value>,
    dimension_default: Option<i64>,
    dimensions: Option<Vec<i64>>,
    time_intervals: Option<Vec<String>>,
    numbers_of_top_n: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TemplateSelectQuery {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TemplateDeleteQuery {
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Format {
    data_type: String,
    weight: f64,
    format: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TemplateHttpTransfer {
    name: String,
    event_type: String,
    method: String,
    algorithm: Option<String>,
    min_token_length: Option<i64>,
    eps: Option<f64>,
    format: Option<Vec<Format>>,
    dimension_default: Option<i64>,
    dimensions: Option<Vec<i64>>,
    time_intervals: Option<Vec<String>>,
    numbers_of_top_n: Option<Vec<i64>>,
}

pub(crate) async fn add_template(
    pool: Data<Pool>,
    payload: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    use template::dsl;
    let bytes = load_payload(payload).await?;
    let new_template: TemplateHttpTransfer = serde_json::from_slice(&bytes)?;

    let format = new_template
        .format
        .and_then(|f| serde_json::to_value(&f).ok());

    let new_template = Template {
        name: new_template.name,
        event_type: new_template.event_type,
        method: new_template.method,
        algorithm: new_template.algorithm,
        min_token_length: new_template.min_token_length,
        eps: new_template.eps,
        format,
        dimension_default: new_template.dimension_default,
        dimensions: new_template.dimensions,
        time_intervals: new_template.time_intervals,
        numbers_of_top_n: new_template.numbers_of_top_n,
    };

    let insert_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        diesel::insert_into(dsl::template)
            .values(new_template)
            .execute(&conn)
            .map_err(Into::into)
    });

    match insert_result {
        Ok(_) => Ok(HttpResponse::Created().into()),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}

pub(crate) async fn update_template(
    pool: Data<Pool>,
    payload: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    use template::dsl;
    let bytes = load_payload(payload).await?;
    let new_template: TemplateHttpTransfer = serde_json::from_slice(&bytes)?;

    let format = new_template
        .format
        .and_then(|f| serde_json::to_value(&f).ok());

    let name = new_template.name.clone();
    let new_template = Template {
        name: new_template.name,
        event_type: new_template.event_type,
        method: new_template.method,
        algorithm: new_template.algorithm,
        min_token_length: new_template.min_token_length,
        eps: new_template.eps,
        format,
        dimension_default: new_template.dimension_default,
        dimensions: new_template.dimensions,
        time_intervals: new_template.time_intervals,
        numbers_of_top_n: new_template.numbers_of_top_n,
    };

    let update_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        diesel::update(dsl::template)
            .filter(dsl::name.eq(&name))
            .set(new_template)
            .execute(&conn)
            .map_err(Into::into)
    });

    match update_result {
        Ok(_) => Ok(HttpResponse::Created().into()),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}

pub(crate) async fn delete_template(
    pool: Data<Pool>,
    query: Query<TemplateDeleteQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use template::dsl;
    let delete_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        diesel::delete(dsl::template.filter(dsl::name.eq(&query.name)))
            .execute(&conn)
            .map_err(Into::into)
    });

    match delete_result {
        Ok(0) => Ok(HttpResponse::BadRequest().into()),
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(build_http_500_response(&e)),
    }
}

pub(crate) async fn get_template(
    pool: Data<Pool>,
    query: Query<TemplateSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<Vec<Template>, Error> =
        pool.get()
            .map_err(Into::into)
            .and_then(|conn| match &query.name {
                Some(name) => template::dsl::template
                    .select((
                        template::dsl::name,
                        template::dsl::event_type,
                        template::dsl::method,
                        template::dsl::algorithm,
                        template::dsl::min_token_length,
                        template::dsl::eps,
                        template::dsl::format,
                        template::dsl::dimension_default,
                        template::dsl::dimensions,
                        template::dsl::time_intervals,
                        template::dsl::numbers_of_top_n,
                    ))
                    .filter(template::dsl::name.eq(name))
                    .order_by(template::dsl::name.asc())
                    .load::<Template>(&conn)
                    .map_err(Into::into),
                None => template::dsl::template
                    .select((
                        template::dsl::name,
                        template::dsl::event_type,
                        template::dsl::method,
                        template::dsl::algorithm,
                        template::dsl::min_token_length,
                        template::dsl::eps,
                        template::dsl::format,
                        template::dsl::dimension_default,
                        template::dsl::dimensions,
                        template::dsl::time_intervals,
                        template::dsl::numbers_of_top_n,
                    ))
                    .order_by(template::dsl::name.asc())
                    .load::<Template>(&conn)
                    .map_err(Into::into),
            });

    match query_result {
        Ok(template) => {
            let template: Vec<TemplateHttpTransfer> = template
                .into_iter()
                .map(|t| {
                    let format: Option<Vec<Format>> =
                        t.format.and_then(|f| serde_json::from_value(f).ok());
                    TemplateHttpTransfer {
                        name: t.name,
                        event_type: t.event_type,
                        method: t.method,
                        algorithm: t.algorithm,
                        min_token_length: t.min_token_length,
                        eps: t.eps,
                        format,
                        dimension_default: t.dimension_default,
                        dimensions: t.dimensions,
                        time_intervals: t.time_intervals,
                        numbers_of_top_n: t.numbers_of_top_n,
                    }
                })
                .collect();

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(template))
        }
        Err(e) => Ok(build_http_500_response(&e)),
    }
}
