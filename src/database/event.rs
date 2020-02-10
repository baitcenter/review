use actix_web::{
    http,
    web::{Data, Payload, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::{pg::upsert::excluded, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::schema::event;
use crate::database::{
    build_err_msg, get_data_source_id, kafka_metadata_lookup, load_payload,
    lookup_events_with_no_raw_event, Conn, Error, Pool,
};

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "event"]
pub(crate) struct Event {
    pub(crate) message_id: BigDecimal,
    pub(crate) data_source_id: i32,
    pub(crate) raw_event: Option<Vec<u8>>,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.message_id.cmp(&other.message_id)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}

impl Eq for Event {}

pub(crate) fn add_events(conn: &Conn, events: &[Event]) -> Result<usize, Error> {
    use event::dsl;
    diesel::insert_into(dsl::event)
        .values(events)
        .on_conflict((dsl::message_id, dsl::data_source_id))
        .do_update()
        .set(dsl::raw_event.eq(excluded(dsl::raw_event)))
        .execute(conn)
        .map_err(Into::into)
}

pub(crate) fn build_events(event_ids: &[u64], data_source_id: i32) -> Vec<Event> {
    event_ids
        .iter()
        .filter_map(|event_id| {
            FromPrimitive::from_u64(*event_id).map(|event_id| Event {
                message_id: event_id,
                raw_event: None,
                data_source_id,
            })
        })
        .collect::<Vec<_>>()
}

pub(crate) fn delete_events(conn: &Conn, events: &[Event]) -> Result<usize, Error> {
    use event::dsl;
    conn.transaction::<usize, Error, _>(|| {
        Ok(events
            .iter()
            .filter_map(|event| {
                diesel::delete(
                    dsl::event.filter(
                        dsl::data_source_id
                            .eq(event.data_source_id)
                            .and(dsl::message_id.eq(&event.message_id)),
                    ),
                )
                .execute(conn)
                .ok()
            })
            .sum())
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct GetEvent {
    message_id: u64,
    raw_event: Option<Vec<u8>>,
}
pub(crate) async fn get_events(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    use event::dsl;

    let data_source = query.get("data_source").and_then(Value::as_str);
    let filter = query
        .get("filter")
        .and_then(Value::as_str)
        .and_then(|f| serde_json::from_str::<Value>(f).ok());
    let message_ids = if let Some(filter) = filter {
        filter
            .get("message_ids")
            .and_then(Value::as_array)
            .map(|f| {
                f.iter()
                    .filter_map(|f| {
                        let message_id = f.as_u64()?;
                        FromPrimitive::from_u64(message_id)
                    })
                    .collect::<Vec<_>>()
            })
    } else {
        None
    };
    if let (Some(data_source), Some(message_ids)) = (data_source, message_ids) {
        if message_ids.len() <= 100 {
            let query_result: Result<Vec<_>, Error> =
                pool.get().map_err(Into::into).and_then(|conn| {
                    if let Ok(data_source_id) = get_data_source_id(&conn, data_source) {
                        conn.transaction::<Vec<GetEvent>, Error, _>(|| {
                            Ok(message_ids
                                .into_iter()
                                .filter_map(|id| {
                                    let message_id: BigDecimal = FromPrimitive::from_u64(id)?;
                                    let result = dsl::event
                                        .filter(
                                            dsl::data_source_id
                                                .eq(data_source_id)
                                                .and(dsl::message_id.eq(message_id)),
                                        )
                                        .select(dsl::raw_event)
                                        .get_result::<Option<Vec<u8>>>(&conn);

                                    match result {
                                        Ok(Some(raw_event)) => Some(GetEvent {
                                            message_id: id,
                                            raw_event: Some(raw_event),
                                        }),
                                        _ => None,
                                    }
                                })
                                .collect::<Vec<_>>())
                        })
                    } else {
                        // if data_source is not found, return a response with empty body
                        Ok(Vec::new())
                    }
                });

            match query_result {
                Ok(data) => Ok(HttpResponse::Ok()
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .json(data)),
                Err(e) => Ok(HttpResponse::InternalServerError()
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(build_err_msg(&e))),
            }
        } else {
            let message = json!({
                "message": "The number of message_id must be less than or equal to 100.",
            })
            .to_string();
            Ok(HttpResponse::BadRequest()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(message))
        }
    } else {
        let message = json!({
            "message": "Missing required query parameters.",
        })
        .to_string();
        Ok(HttpResponse::BadRequest()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(message))
    }
}

pub(crate) async fn get_events_with_no_raw_event(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let data_source_id = query
        .get("data_source_id")
        .and_then(Value::as_str)
        .and_then(|data_source_id| data_source_id.parse::<i32>().ok());

    if let Some(data_source_id) = data_source_id {
        let query_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
            let message_ids = diesel::select(lookup_events_with_no_raw_event(data_source_id))
                .get_results::<BigDecimal>(&conn)?;
            let mut kafka_metadata = Vec::<(u64, u64)>::new();
            if !message_ids.is_empty() {
                let mut message_ids_cloned = message_ids.clone();
                message_ids_cloned.sort();
                while let Some(metadata) =
                    kafka_metadata_lookup(&conn, data_source_id, &message_ids_cloned[0])
                {
                    if let Some(upper_value) =
                        bigdecimal::FromPrimitive::from_u64(metadata.0) as Option<BigDecimal>
                    {
                        message_ids_cloned.retain(|v| upper_value < *v);
                    } else {
                        break;
                    }

                    kafka_metadata.push((metadata.1, metadata.2));
                    if message_ids_cloned.is_empty() {
                        break;
                    }
                }
            }

            let data = json!({
                "message_ids": message_ids,
                "metadata": kafka_metadata
            })
            .to_string();

            Ok(data)
        });

        match query_result {
            Ok(data) => Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(data)),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
    } else {
        Ok(HttpResponse::BadRequest().into())
    }
}

pub(crate) async fn update_events(
    pool: Data<Pool>,
    payload: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let bytes = load_payload(payload).await?;
    let events: Vec<Event> = serde_json::from_slice(&bytes)?;
    let query_result: Result<usize, Error> = pool
        .get()
        .map_err(Into::into)
        .and_then(|conn| add_events(&conn, &events));

    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
