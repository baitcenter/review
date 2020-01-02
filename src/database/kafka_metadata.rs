use bigdecimal::BigDecimal;
use diesel::prelude::*;
use serde_json::Value;

use crate::database::{lookup_kafka_metadata, Conn};

pub(crate) fn kafka_metadata_lookup(
    conn: &Conn,
    data_source_id: i32,
    message_id: &BigDecimal,
) -> Option<(u64, u64, u64)> {
    diesel::select(lookup_kafka_metadata(data_source_id, message_id))
        .get_result::<Option<Value>>(conn)
        .ok()
        .and_then(|v| v)
        .and_then(|v| {
            if let (Some(message_id), Some(offsets), Some(partition)) = (
                v.get("message_id").and_then(Value::as_u64),
                v.get("offsets").and_then(Value::as_u64),
                v.get("partition").and_then(Value::as_u64),
            ) {
                Some((message_id, partition, offsets))
            } else {
                None
            }
        })
}
