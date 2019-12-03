use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, ToPrimitive};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use eventio::{kafka, Input};
use r2d2::PooledConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;

use super::schema::{cluster, outlier, raw_event};
use crate::database::{build_err_msg, bytes_to_string, data_source, Error, Pool};

#[derive(Debug, Insertable, Queryable, Serialize)]
#[table_name = "raw_event"]
struct RawEventTable {
    id: i32,
    data: String,
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
    data: String,
    data_source_id: i32,
}

#[derive(Queryable)]
struct UpdateRawEventId {
    id: i32,
    data: String,
    is_cluster: bool,
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn add_raw_events(
    pool: Data<Pool>,
    query: Query<RawEventQuery>,
    kafka_url: Data<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let query = query.into_inner();
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e)))
        }
    };
    if let (Ok(event_ids_from_clusters), Ok(event_ids_from_outliers), Ok(data_source_id)) = (
        event_ids_from_cluster(&conn, &query.data_source),
        event_ids_from_outlier(&conn, &query.data_source),
        data_source::id(&conn, &query.data_source),
    ) {
        let (data_tx, data_rx) = crossbeam_channel::bounded(256);
        let (ack_tx, ack_rx) = crossbeam_channel::bounded(256);
        let event_input = match kafka::Input::new(
            data_tx,
            ack_rx,
            vec![kafka_url.get_ref().into()],
            "REviewd".into(),
            "REview".into(),
            query.data_source,
            usize::max_value(),
        ) {
            Ok(input) => input,
            Err(_) => return Ok(HttpResponse::InternalServerError().into()),
        };
        let in_thread = thread::spawn(move || match event_input.run() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("{}", e)),
        });

        let mut update_lists = Vec::<UpdateRawEventId>::new();
        let mut raw_events: Vec<InsertRawEvent> = Vec::new();
        let mut event_count = 0;
        {
            let ack_tx = ack_tx;
            for ev in data_rx {
                event_count += 1;
                let id = ev.entry.time;
                if let Some(c) = event_ids_from_clusters.iter().find(|d| id == d.1) {
                    if let Some(raw) = ev.entry.record.get("message") {
                        let raw = bytes_to_string(raw);
                        update_lists.push(UpdateRawEventId {
                            id: c.0,
                            data: raw.clone(),
                            is_cluster: true,
                        });
                        raw_events.push(InsertRawEvent {
                            data: raw,
                            data_source_id,
                        });
                    }
                } else if let Some(o) = event_ids_from_outliers.iter().find(|d| id == d.1) {
                    if let Some(raw) = ev.entry.record.get("message") {
                        let raw = bytes_to_string(raw);
                        update_lists.push(UpdateRawEventId {
                            id: o.0,
                            data: raw.clone(),
                            is_cluster: false,
                        });
                        raw_events.push(InsertRawEvent {
                            data: raw,
                            data_source_id,
                        });
                    }
                }
                if ack_tx.send(ev.loc).is_err() {
                    return Ok(HttpResponse::InternalServerError().into());
                }
                if event_count >= query.max_event_count {
                    break;
                }
            }
        }
        let thread_result = in_thread.join();

        if !raw_events.is_empty() {
            let _ = diesel::insert_into(raw_event::dsl::raw_event)
                .values(&raw_events)
                .execute(&conn);
            if let Ok(raw_events) = events_by_data_source_id(&conn, data_source_id) {
                let raw_events = raw_events
                    .into_iter()
                    .map(|e| (e.1, e.0))
                    .collect::<HashMap<String, i32>>();
                for u in update_lists {
                    if let Some(raw_event_id) = raw_events.get(&u.data) {
                        if u.is_cluster {
                            use cluster::dsl;
                            let _ = diesel::update(dsl::cluster.filter(dsl::id.eq(u.id)))
                                .set(dsl::raw_event_id.eq(raw_event_id))
                                .execute(&*conn);
                        } else {
                            use outlier::dsl;
                            let _ = diesel::update(dsl::outlier.filter(dsl::id.eq(u.id)))
                                .set(dsl::raw_event_id.eq(raw_event_id))
                                .execute(&*conn);
                        }
                    }
                }
            }
        }

        if thread_result.is_err() {
            return Ok(HttpResponse::InternalServerError().into());
        }
    }

    Ok(HttpResponse::Ok().into())
}

pub(crate) fn empty_event_id(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    data_source_id: i32,
) -> Result<i32, Error> {
    use raw_event::dsl;
    dsl::raw_event
        .filter(
            dsl::data_source_id
                .eq(data_source_id)
                .and(dsl::data.eq("-")),
        )
        .select(dsl::id)
        .first::<i32>(conn)
        .map_err(Into::into)
}

fn events_by_data_source_id(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    data_source_id: i32,
) -> Result<Vec<(i32, String)>, Error> {
    use raw_event::dsl;
    dsl::raw_event
        .filter(dsl::data_source_id.eq(data_source_id))
        .select((dsl::id, dsl::data))
        .load::<(i32, String)>(conn)
        .map_err(Into::into)
}

fn event_ids_from_cluster(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    data_source: &str,
) -> Result<Vec<(i32, u64)>, Error> {
    use cluster::dsl;
    if let Ok(data_source_id) = data_source::id(conn, data_source) {
        let raw_event_id = empty_event_id(conn, data_source_id).unwrap_or_default();
        dsl::cluster
            .filter(
                dsl::data_source_id
                    .eq(data_source_id)
                    .and(dsl::raw_event_id.eq(raw_event_id)),
            )
            .select((dsl::id, dsl::event_ids))
            .load::<(i32, Option<Vec<BigDecimal>>)>(conn)
            .map_err(Into::into)
            .map(|data| {
                data.into_iter()
                    .filter_map(|d| {
                        let id = d.0;
                        d.1.and_then(|event_ids| event_ids.into_iter().max())
                            .and_then(|e| ToPrimitive::to_u64(&e))
                            .map(|e| (id, e))
                    })
                    .collect()
            })
    } else {
        Err(Error::RecordNotExist)
    }
}

fn event_ids_from_outlier(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    data_source: &str,
) -> Result<Vec<(i32, u64)>, Error> {
    use outlier::dsl;
    if let Ok(data_source_id) = data_source::id(conn, data_source) {
        let raw_event_id = empty_event_id(conn, data_source_id).unwrap_or_default();
        dsl::outlier
            .filter(
                dsl::data_source_id
                    .eq(data_source_id)
                    .and(dsl::raw_event_id.eq(raw_event_id)),
            )
            .select((dsl::id, dsl::event_ids))
            .load::<(i32, Vec<BigDecimal>)>(conn)
            .map_err(Into::into)
            .map(|data| {
                data.into_iter()
                    .filter_map(|d| {
                        let id = d.0;
                        d.1.into_iter()
                            .max()
                            .and_then(|e| ToPrimitive::to_u64(&e))
                            .map(|e| (id, e))
                    })
                    .collect()
            })
    } else {
        Err(Error::RecordNotExist)
    }
}
