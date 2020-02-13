use actix_web::{
    http,
    web::{Data, Payload, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use super::schema::outlier;
use crate::database::*;

#[derive(Debug, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "outlier"]
struct Outlier {
    id: i32,
    raw_event: Vec<u8>,
    data_source_id: i32,
    event_ids: Vec<BigDecimal>,
    size: BigDecimal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OutlierUpdate {
    id: i32,
    outlier: Vec<u8>,
    data_source: String,
    data_source_type: String,
    event_ids: Vec<u64>,
    size: usize,
}

pub(crate) async fn delete_outliers(
    pool: Data<Pool>,
    payload: Payload,
    data_source: Query<DataSourceQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use outlier::dsl;

    let bytes = load_payload(payload).await?;
    let outliers: Vec<Vec<u8>> = serde_json::from_slice(&bytes)?;
    let data_source = data_source.into_inner();
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e)))
        }
    };
    if let Ok(data_source_id) = get_data_source_id(&conn, &data_source.data_source) {
        let mut query = dsl::outlier.into_boxed();
        for outlier in &outliers {
            query = query.or_filter(dsl::raw_event.eq(outlier));
        }
        query = query.filter(dsl::data_source_id.eq(data_source_id));
        let _ = query
            .select(dsl::event_ids)
            .load::<Vec<BigDecimal>>(&conn)
            .map_err(Into::into)
            .and_then(|event_ids| {
                let events = event_ids
                    .into_iter()
                    .flatten()
                    .map(|message_id| Event {
                        message_id,
                        data_source_id,
                        raw_event: None,
                    })
                    .collect::<Vec<_>>();
                delete_events(&conn, &events)
            });

        let query_result: Result<usize, Error> = conn.build_transaction().read_write().run(|| {
            Ok(outliers
                .iter()
                .filter_map(|outlier| {
                    diesel::delete(
                        dsl::outlier.filter(
                            dsl::data_source_id
                                .eq(data_source_id)
                                .and(dsl::raw_event.eq(outlier)),
                        ),
                    )
                    .execute(&conn)
                    .ok()
                })
                .sum())
        });

        match query_result {
            Ok(_) => Ok(HttpResponse::Ok().into()),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
    } else {
        Ok(HttpResponse::InternalServerError().into())
    }
}

pub(crate) async fn get_outliers(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let default_per_page = 10;
    let max_per_page = 100;
    let outlier_schema =
        "(outlier INNER JOIN data_source ON outlier.data_source_id = data_source.id)";
    let select = query
        .get("select")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<HashMap<String, bool>>(s).ok())
        .map(|s| {
            s.iter()
                .filter(|s| *s.1)
                .filter_map(|s| match s.0.to_lowercase().as_str() {
                    "id" => Some("outlier.id"),
                    "outlier" => Some("right((outlier.raw_event)::TEXT, -2) as outlier"),
                    "data_source" => Some("data_source.topic_name as data_source"),
                    "size" => Some("outlier.size"),
                    "event_ids" => Some("outlier.event_ids"),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            vec![
                "right((outlier.raw_event)::TEXT, -2) as outlier",
                "data_source.topic_name as data_source",
                "outlier.size",
                "outlier.event_ids",
            ]
        });
    let filter = query
        .get("filter")
        .and_then(Value::as_str)
        .and_then(|f| serde_json::from_str::<Value>(f).ok());
    let where_clause = if let Some(filter) = filter {
        filter.get("data_source")
        .and_then(Value::as_array)
        .map(|f| {
            let mut where_clause = String::new();
            for (index, f) in f.iter().enumerate() {
                if let Some(f) = f.as_str() {
                    let filter = format!("Outlier.data_source_id = (SELECT id FROM data_source WHERE topic_name = '{}')", f);
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
    let page = GetQuery::get_page(&query);
    let per_page = GetQuery::get_per_page(&query, max_per_page).unwrap_or_else(|| default_per_page);
    let orderby = query
        .get("orderby")
        .and_then(Value::as_str)
        .and_then(|column_name| match column_name.to_lowercase().as_str() {
            "outlier" => Some("outlier.raw_event"),
            "data_source" => Some("data_source.topic_name"),
            "size" => Some("outlier.size"),
            "event_ids" => Some("outlier.event_ids"),
            _ => None,
        });
    let order = if orderby.is_some() {
        GetQuery::get_order(&query)
    } else {
        None
    };

    match pool.get() {
        Ok(conn) => GetQuery::build_response(
            select,
            outlier_schema,
            where_clause,
            page,
            per_page,
            orderby,
            order,
            &conn,
        ),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

pub(crate) async fn update_outliers(
    pool: Data<Pool>,
    payload: Payload,
    max_event_id_num: Data<Mutex<usize>>,
) -> Result<HttpResponse, actix_web::Error> {
    let bytes = load_payload(payload).await?;
    let outlier_update: Vec<OutlierUpdate> = serde_json::from_slice(&bytes)?;
    let max_event_id_num: BigDecimal = match max_event_id_num.lock() {
        Ok(num) => FromPrimitive::from_usize(*num).expect("should be fine"),
        Err(e) => {
            error!(
                "Failed to acquire lock: {}. Use default max_event_id_number 25",
                e
            );
            FromPrimitive::from_usize(25).expect("should be fine")
        }
    };

    let query_result: Result<i32, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let outlier_update_clone = outlier_update.clone();
        let result = conn.transaction::<i32, Error, _>(|| {
            Ok(outlier_update_clone
                .into_iter()
                .filter_map(|o| {
                    let event_ids = o
                        .event_ids
                        .iter()
                        .filter_map(|event_id| FromPrimitive::from_u64(*event_id))
                        .collect::<Vec<BigDecimal>>();
                    if event_ids.is_empty() {
                        return None;
                    }
                    let size: BigDecimal = FromPrimitive::from_usize(o.size)?;
                    let query_result = diesel::select(attempt_outlier_upsert(
                        max_event_id_num.clone(),
                        o.id,
                        o.outlier,
                        o.data_source,
                        o.data_source_type,
                        event_ids,
                        size,
                    ))
                    .get_result::<i32>(&conn);
                    if let Err(e) = &query_result {
                        log::error!("Failed to insert/update an outlier: {}", e);
                    }
                    query_result.ok()
                })
                .sum())
        });
        update_events(&conn, &outlier_update);
        result
    });

    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

fn update_events(conn: &Conn, outlier_update: &[OutlierUpdate]) {
    let mut new_events = outlier_update
        .iter()
        .filter_map(|outlier| {
            if let Ok(data_source_id) = get_data_source_id(conn, &outlier.data_source) {
                Some(build_events(&outlier.event_ids, data_source_id))
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    if !new_events.is_empty() {
        new_events.sort();
        new_events.dedup();
        if let Err(e) = add_events(&conn, &new_events) {
            log::error!("An error occurs while inserting events: {}", e);
        }
    }
}
