use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use diesel::prelude::*;
use futures::{future, prelude::*};
use remake::event::Identifiable;
use remake::stream::EventorKafka;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{cluster, outlier, raw_event};
use crate::database::{get_data_source_id, DatabaseError, Error, ErrorKind, Pool};

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "raw_event"]
struct RawEventTable {
    id: i32,
    data: Vec<u8>,
    data_source_id: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawEventQuery {
    data_source: String,
    max_event_count: usize,
}

#[derive(Insertable)]
#[table_name = "raw_event"]
struct InsertRawEvent {
    data: Vec<u8>,
    data_source_id: i32,
}

#[derive(Queryable)]
struct UpdateRawEventId {
    id: i32,
    data: Vec<u8>,
    is_cluster: bool,
}

pub(crate) fn add_raw_events(
    pool: Data<Pool>,
    query: Query<RawEventQuery>,
    kafka_url: Data<String>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query = query.into_inner();
    if let (
        Ok(event_ids_from_clusters),
        Ok(event_ids_from_outliers),
        Ok(consumer),
        Ok(data_source_id),
    ) = (
        get_event_ids_from_cluster(&pool, &query.data_source),
        get_event_ids_from_outlier(&pool, &query.data_source),
        EventorKafka::new(&kafka_url, &query.data_source, "REviewd"),
        get_data_source_id(&pool, &query.data_source),
    ) {
        let mut update_lists = Vec::<UpdateRawEventId>::new();
        let raw_events: Vec<InsertRawEvent> =
            EventorKafka::fetch_messages(&consumer, query.max_event_count)
                .into_iter()
                .filter_map(|data| {
                    if let Some(c) = event_ids_from_clusters.iter().find(|d| data.id() == d.1) {
                        update_lists.push(UpdateRawEventId {
                            id: c.0,
                            data: data.data().to_vec(),
                            is_cluster: true,
                        });
                        Some(InsertRawEvent {
                            data: data.data().to_vec(),
                            data_source_id,
                        })
                    } else if let Some(o) =
                        event_ids_from_outliers.iter().find(|d| data.id() == d.1)
                    {
                        update_lists.push(UpdateRawEventId {
                            id: o.0,
                            data: data.data().to_vec(),
                            is_cluster: false,
                        });
                        Some(InsertRawEvent {
                            data: data.data().to_vec(),
                            data_source_id,
                        })
                    } else {
                        None
                    }
                })
                .collect();

        if !raw_events.is_empty() {
            let _: Result<(), Error> = pool.get().map_err(Into::into).and_then(|conn| {
                let _ = diesel::insert_into(raw_event::dsl::raw_event)
                    .values(&raw_events)
                    .execute(&*conn);
                if let Ok(raw_events) = get_raw_events_by_data_source_id(&pool, data_source_id) {
                    let raw_events = raw_events
                        .into_iter()
                        .map(|e| (e.1, e.0))
                        .collect::<HashMap<Vec<u8>, i32>>();
                    for u in update_lists {
                        if let Some(raw_event_id) = raw_events.get(&u.data) {
                            if u.is_cluster {
                                use cluster::dsl;
                                let _ = diesel::update(dsl::cluster.filter(dsl::id.eq(u.id)))
                                    .set(dsl::raw_event_id.eq(Some(raw_event_id)))
                                    .execute(&*conn);
                            } else {
                                use outlier::dsl;
                                let _ = diesel::update(dsl::outlier.filter(dsl::id.eq(u.id)))
                                    .set(dsl::raw_event_id.eq(Some(raw_event_id)))
                                    .execute(&*conn);
                            }
                        }
                    }
                }
                Ok(())
            });
        }
    }

    future::result(Ok(HttpResponse::Ok().into()))
}

pub(crate) fn get_raw_events_by_data_source_id(
    pool: &Data<Pool>,
    data_source_id: i32,
) -> Result<Vec<(i32, Vec<u8>)>, Error> {
    use raw_event::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::raw_event
            .filter(dsl::data_source_id.eq(data_source_id))
            .select((dsl::id, dsl::data))
            .load::<(i32, Vec<u8>)>(&conn)
            .map_err(Into::into)
    })
}

pub(crate) fn get_raw_event_by_raw_event_id(
    pool: &Data<Pool>,
    raw_event_id: i32,
) -> Result<Vec<u8>, Error> {
    use raw_event::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::raw_event
            .filter(dsl::id.eq(raw_event_id))
            .select(dsl::data)
            .first::<Vec<u8>>(&conn)
            .map_err(Into::into)
    })
}

fn get_event_ids_from_cluster(
    pool: &Data<Pool>,
    data_source: &str,
) -> Result<Vec<(i32, u64)>, Error> {
    use cluster::dsl;
    if let Ok(data_source_id) = get_data_source_id(&pool, data_source) {
        pool.get()
            .map_err(Into::into)
            .and_then(|conn| {
                dsl::cluster
                    .filter(
                        dsl::data_source_id
                            .eq(data_source_id)
                            .and(dsl::raw_event_id.is_null()),
                    )
                    .select((dsl::id, dsl::event_ids))
                    .load::<(i32, Option<Vec<u8>>)>(&conn)
                    .map_err(Into::into)
            })
            .map(|data| {
                data.into_iter()
                    .filter_map(|d| {
                        let id = d.0;
                        d.1.and_then(|event_ids| {
                            rmp_serde::decode::from_slice::<Vec<u64>>(&event_ids).ok()
                        })
                        .filter(|event_ids| !event_ids.is_empty())
                        .map(|mut event_ids| {
                            event_ids.sort();
                            (id, event_ids[event_ids.len() - 1])
                        })
                    })
                    .collect()
            })
    } else {
        Err(ErrorKind::DatabaseTransactionError(DatabaseError::RecordNotExist).into())
    }
}

fn get_event_ids_from_outlier(
    pool: &Data<Pool>,
    data_source: &str,
) -> Result<Vec<(i32, u64)>, Error> {
    use outlier::dsl;
    if let Ok(data_source_id) = get_data_source_id(&pool, data_source) {
        pool.get()
            .map_err(Into::into)
            .and_then(|conn| {
                dsl::outlier
                    .filter(
                        dsl::data_source_id
                            .eq(data_source_id)
                            .and(dsl::raw_event_id.is_null()),
                    )
                    .select((dsl::id, dsl::event_ids))
                    .load::<(i32, Vec<u8>)>(&conn)
                    .map_err(Into::into)
            })
            .map(|data| {
                data.into_iter()
                    .filter_map(|d| {
                        let id = d.0;
                        rmp_serde::decode::from_slice::<Vec<u64>>(&d.1)
                            .ok()
                            .filter(|examples| !examples.is_empty())
                            .map(|mut examples| {
                                examples.sort();
                                (id, examples[examples.len() - 1])
                            })
                    })
                    .collect()
            })
    } else {
        Err(ErrorKind::DatabaseTransactionError(DatabaseError::RecordNotExist).into())
    }
}
