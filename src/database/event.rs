use actix_web::{
    http,
    web::{Data, Json, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::{pg::upsert::excluded, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::schema::event;
use crate::database::{
    build_err_msg, get_data_source_id, kafka_metadata_lookup, lookup_events_with_no_raw_event,
    Conn, Error, Pool,
};

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "event"]
pub(crate) struct Event {
    pub(crate) message_id: BigDecimal,
    pub(crate) data_source_id: i32,
    pub(crate) raw_event: Option<String>,
}

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

pub(crate) fn delete_events(conn: &Conn, events: &[Event]) -> Result<usize, Error> {
    use event::dsl;
    conn.build_transaction().read_write().run(|| {
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
pub(crate) struct GetEvent {
    message_id: u64,
    data_source: String,
    raw_event: Option<String>,
}
pub(crate) async fn get_events(
    pool: Data<Pool>,
    events: Json<Vec<GetEvent>>,
) -> Result<HttpResponse, actix_web::Error> {
    use event::dsl;

    let events = events.into_inner();
    // Accept only HTTP requests that request less than 100 raw_events
    if events.len() > 100 {
        Ok(HttpResponse::BadRequest().into())
    } else {
        let query_result: Result<Vec<_>, Error> = pool.get().map_err(Into::into).and_then(|conn| {
            conn.build_transaction().read_only().run(|| {
                Ok(events
                    .into_iter()
                    .filter_map(|event| {
                        let message_id: BigDecimal = FromPrimitive::from_u64(event.message_id)?;
                        let data_source_id = get_data_source_id(&conn, &event.data_source).ok()?;
                        let result = dsl::event
                            .filter(
                                dsl::data_source_id
                                    .eq(data_source_id)
                                    .and(dsl::message_id.eq(message_id)),
                            )
                            .select(dsl::raw_event)
                            .get_result::<Option<String>>(&conn);

                        match result {
                            Ok(Some(raw_event)) => Some(GetEvent {
                                message_id: event.message_id,
                                data_source: event.data_source,
                                raw_event: Some(raw_event),
                            }),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>())
            })
        });

        match query_result {
            Ok(data) => Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(data)),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
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
    events: Json<Vec<Event>>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let events = events.into_inner();
        add_events(&conn, &events)
    });

    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
