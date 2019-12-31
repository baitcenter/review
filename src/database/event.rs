use bigdecimal::BigDecimal;
use diesel::{pg::upsert::excluded, prelude::*};
use serde::{Deserialize, Serialize};

use super::schema::event;
use crate::database::{Conn, Error};

#[derive(Debug, Insertable, Serialize, Deserialize)]
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
