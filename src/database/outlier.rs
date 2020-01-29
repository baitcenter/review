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
    outlier: Vec<u8>,
    data_source: String,
    data_source_type: String,
    event_ids: Vec<u64>,
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

    let query_result: Result<Vec<GetQueryData>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            GetQuery::new(
                select,
                outlier_schema,
                where_clause,
                page,
                per_page,
                orderby,
                order,
            )
            .get_results::<GetQueryData>(&conn)
            .map_err(Into::into)
        });

    GetQuery::build_response(&query, per_page, query_result)
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn update_outliers(
    pool: Data<Pool>,
    payload: Payload,
    max_event_id_num: Data<Mutex<usize>>,
) -> Result<HttpResponse, actix_web::Error> {
    use outlier::dsl;
    let bytes = load_payload(payload).await?;
    let outlier_update: Vec<OutlierUpdate> = serde_json::from_slice(&bytes)?;
    let max_event_id_num = match max_event_id_num.lock() {
        Ok(num) => *num,
        Err(e) => {
            error!(
                "Failed to acquire lock: {}. Use the default max_event_id_number 25",
                e
            );
            25
        }
    };
    let mut deleted_events = Vec::<Event>::new();
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let mut query = dsl::outlier.into_boxed();
        for outlier in &outlier_update {
            if let Ok(data_source_id) = get_data_source_id(&conn, &outlier.data_source) {
                query = query.or_filter(
                    dsl::raw_event
                        .eq(&outlier.outlier)
                        .and(dsl::data_source_id.eq(data_source_id)),
                );
            }
        }
        query
            .load::<Outlier>(&conn)
            .map_err(Into::into)
            .and_then(|outlier_list| {
                let new_outliers = outlier_update
                    .iter()
                    .filter_map(|o| {
                        use std::str::FromStr;
                        if let Some(outlier) = outlier_list
                            .iter()
                            .find(|outlier| o.outlier == outlier.raw_event)
                        {
                            let o_size = FromPrimitive::from_usize(o.event_ids.len()).map_or_else(
                                || outlier.size.clone(),
                                |new_size: BigDecimal| {
                                    // Reset the value of size if it exceeds 20 digits
                                    BigDecimal::from_str("100000000000000000000").ok().map_or(
                                        new_size.clone(),
                                        |max_size| {
                                            if &new_size + &outlier.size < max_size {
                                                &new_size + &outlier.size
                                            } else {
                                                new_size
                                            }
                                        },
                                    )
                                },
                            );
                            let mut event_ids = o
                                .event_ids
                                .iter()
                                .filter_map(|e| FromPrimitive::from_u64(*e))
                                .collect::<Vec<BigDecimal>>();
                            event_ids.extend(outlier.event_ids.clone());
                            let (event_ids, deleted_event_ids) =
                                if event_ids.len() > max_event_id_num {
                                    event_ids.sort();
                                    let (deleted_event_ids, event_ids) =
                                        event_ids.split_at(event_ids.len() - max_event_id_num);
                                    (event_ids.to_vec(), Some(deleted_event_ids.to_vec()))
                                } else {
                                    (event_ids, None)
                                };
                            if let Some(event_ids) = deleted_event_ids {
                                let events = event_ids
                                    .into_iter()
                                    .map(|event_id| Event {
                                        message_id: event_id,
                                        raw_event: None,
                                        data_source_id: outlier.data_source_id,
                                    })
                                    .collect::<Vec<_>>();
                                deleted_events.extend(events);
                            }
                            Some(Outlier {
                                id: outlier.id,
                                raw_event: outlier.raw_event.clone(),
                                data_source_id: outlier.data_source_id,
                                event_ids,
                                size: o_size,
                            })
                        } else {
                            let mut event_ids = o
                                .event_ids
                                .iter()
                                .filter_map(|e| FromPrimitive::from_u64(*e))
                                .collect::<Vec<BigDecimal>>();
                            let event_ids = if event_ids.len() > max_event_id_num {
                                event_ids.sort();
                                let (_, event_ids) =
                                    event_ids.split_at(o.event_ids.len() - max_event_id_num);
                                event_ids.to_vec()
                            } else {
                                event_ids
                            };
                            let size: BigDecimal = FromPrimitive::from_usize(o.event_ids.len())
                                .unwrap_or_else(|| {
                                    FromPrimitive::from_usize(1).unwrap_or_default()
                                });
                            let data_source_id = get_data_source_id(&conn, &o.data_source)
                                .unwrap_or_else(|_| {
                                    data_source::add(&conn, &o.data_source, &o.data_source_type)
                                        .unwrap_or_else(|_| {
                                            get_data_source_id(&conn, &o.data_source)
                                                .unwrap_or_default()
                                        })
                                });
                            if data_source_id == 0 {
                                None
                            } else {
                                Some(Outlier {
                                    id: 0,
                                    raw_event: o.outlier.clone(),
                                    data_source_id,
                                    event_ids,
                                    size,
                                })
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                update_events(&conn, &outlier_update, &deleted_events);
                conn.transaction::<usize, Error, _>(|| {
                    Ok(new_outliers
                        .iter()
                        .filter_map(|outlier| {
                            if outlier.id == 0 {
                                let result = diesel::insert_into(dsl::outlier)
                                    .values((
                                        dsl::raw_event.eq(&outlier.raw_event),
                                        dsl::data_source_id.eq(outlier.data_source_id),
                                        dsl::event_ids.eq(&outlier.event_ids),
                                        dsl::size.eq(&outlier.size),
                                    ))
                                    .execute(&conn);
                                if let Err(e) = result {
                                    error!("An error occurs while inserting outliers: {}", e);
                                    None
                                } else {
                                    Some(1)
                                }
                            } else {
                                let result = diesel::update(dsl::outlier)
                                    .filter(dsl::id.eq(outlier.id))
                                    .set((
                                        dsl::event_ids.eq(&outlier.event_ids),
                                        dsl::size.eq(&outlier.size),
                                    ))
                                    .execute(&conn);
                                if let Err(e) = result {
                                    error!("An error occurs while updating outliers: {}", e);
                                    None
                                } else {
                                    Some(1)
                                }
                            }
                        })
                        .sum())
                })
            })
    });

    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

fn update_events(conn: &Conn, outlier_update: &[OutlierUpdate], deleted_events: &[Event]) {
    let new_events = outlier_update
        .iter()
        .filter_map(|outlier| {
            if let Ok(data_source_id) = get_data_source_id(conn, &outlier.data_source) {
                Some(
                    outlier
                        .event_ids
                        .iter()
                        .filter_map(|event_id| {
                            FromPrimitive::from_u64(*event_id).map(|event_id| Event {
                                message_id: event_id,
                                raw_event: None,
                                data_source_id,
                            })
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    if !new_events.is_empty() {
        if let Err(e) = add_events(&conn, &new_events) {
            log::error!("An error occurs while inserting events: {}", e);
        }
    }
    if !deleted_events.is_empty() {
        if let Err(e) = delete_events(&conn, &deleted_events) {
            log::error!("An error occurs while deleting events: {}", e);
        }
    }
}
