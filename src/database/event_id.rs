use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::prelude::*;
use serde_json::Value;
use std::sync::Mutex;

use crate::database::{attempt_event_ids_update, build_err_msg, Conn, Pool};

pub(crate) async fn get_max_event_id_num(max_event_id_num: Data<Mutex<usize>>) -> HttpResponse {
    let max_event_id_num = max_event_id_num.lock().unwrap();
    let message = format!(r#"{{"max_event_id_num": {} }}"#, max_event_id_num);

    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(message)
}

pub(crate) fn update_event_ids(conn: &Conn, max_event_id_num: BigDecimal) -> Result<(), String> {
    diesel::select(attempt_event_ids_update(max_event_id_num))
        .execute(conn)
        .map_err(|e| format!("Failed to run attempt_event_ids_update: {}", e))
        .map(|_| ())
}

pub(crate) async fn update_max_event_id_num(
    pool: Data<Pool>,
    max_event_id_num: Data<Mutex<usize>>,
    query: Query<Value>,
) -> HttpResponse {
    match query
        .get("max_event_id_num")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<usize>().ok())
    {
        Some(new_max_event_id_num) => match max_event_id_num.lock() {
            Ok(mut max_event_id_num) => {
                if *max_event_id_num > new_max_event_id_num {
                    if let (Ok(conn), Some(num)) =
                        (pool.get(), FromPrimitive::from_usize(new_max_event_id_num))
                    {
                        let _ = update_event_ids(&conn, num);
                    }
                }
                *max_event_id_num = new_max_event_id_num;
                HttpResponse::Ok().into()
            }
            Err(e) => HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e)),
        },
        None => HttpResponse::BadRequest().into(),
    }
}
