use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use eventio::fluentd::ForwardMode;
use kafka::client::{FetchPartition, KafkaClient};
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Bound::Included;

use crate::database::{Event, KafkaMetadata};

pub(crate) async fn fetch_kafka_metadata(
    kafka_url: String,
    topic_name: String,
    data_source_id: i32,
    reviewd_addr: String,
    max_offset_count: usize,
) -> Result<(), String> {
    let mut consumer = Consumer::from_hosts(vec![kafka_url])
        .with_group("REviewd2".into()) // Remember to update this to "REviewd" when raw_event.rs in reviewd is removed
        .with_fallback_offset(FetchOffset::Earliest)
        .with_fetch_max_bytes_per_partition(10_000_000)
        .with_offset_storage(GroupOffsetStorage::Kafka)
        .with_client_id("REview".into())
        .with_topic(topic_name)
        .create()
        .map_err(|e| format!("Failed to create Kafka consumer: {}", e))?;

    let mut offset_count = 0;
    let mut metadata = Vec::<KafkaMetadata>::new();
    loop {
        let messagesets = consumer
            .poll()
            .map_err(|e| format!("Failed to consume kafka messages: {}", e))?;
        if messagesets.is_empty() {
            break;
        }
        for msgset in messagesets.iter() {
            let partition = msgset.partition();
            let result = (0..msgset.messages().len())
                .filter_map(|index| {
                    let msg = msgset.messages().get(index)?;
                    let fwd_msg: ForwardMode = rmp_serde::from_slice(msg.value).ok()?;
                    let first: BigDecimal = FromPrimitive::from_u64(fwd_msg.entries[0].time)?;
                    let last: BigDecimal =
                        FromPrimitive::from_u64(fwd_msg.entries[fwd_msg.entries.len() - 1].time)?;
                    offset_count += 1;

                    Some(KafkaMetadata {
                        data_source_id,
                        partition,
                        offsets: msg.offset,
                        message_ids: (Included(first), Included(last)),
                    })
                })
                .collect::<Vec<_>>();

            if !result.is_empty() {
                metadata.extend(result);
                if let Err(e) = consumer.consume_messageset(msgset) {
                    log::error!("Failed to mark messages consumed: {}", e);
                }
            }
        }
        if offset_count >= max_offset_count {
            break;
        }
    }
    if !metadata.is_empty() {
        let api_url = format!("http://{}/api/kafka_metadata", reviewd_addr);
        if send_http_put_request(&metadata, &api_url).await.is_ok() {
            consumer
                .commit_consumed()
                .map_err(|e| format!("Failed to send commit message(s): {}", e))?;
        }
    }

    Ok(())
}

pub(crate) async fn fetch_raw_events(
    kafka_url: String,
    topic_name: String,
    data_source_id: i32,
    reviewd_addr: String,
    max_offset_count: usize,
) -> Result<(), String> {
    let api_url = format!(
        "http://{}/api/event/no_raw_events?data_source_id={}",
        reviewd_addr, data_source_id
    );
    match send_http_get_request(&api_url).await {
        Ok(data) => {
            if let Ok(data) = serde_json::from_str::<Value>(&data) {
                let message_ids = data
                    .get("message_ids")
                    .and_then(|value| serde_json::from_value::<Vec<BigDecimal>>(value.clone()).ok())
                    .map(|value| {
                        value
                            .iter()
                            .filter_map(|v| ToPrimitive::to_u64(v))
                            .collect::<Vec<_>>()
                    });
                let metadata = data
                    .get("metadata")
                    .and_then(|v| serde_json::from_value::<Vec<(i32, i64)>>(v.clone()).ok());
                if let (Some(message_ids), Some(metadata)) = (message_ids, metadata) {
                    let metadata = if metadata.len() > max_offset_count {
                        let (metadata, _) = metadata.split_at(max_offset_count);
                        metadata.to_vec()
                    } else {
                        metadata
                    };
                    let mut client = KafkaClient::new(vec![kafka_url]);
                    let entries = metadata
                        .iter()
                        .filter_map(|(partition, offsets)| {
                            let _ = client.load_metadata(&[&topic_name]);
                            let req = &FetchPartition::new(&topic_name, *partition, *offsets)
                                .with_max_bytes(1_000_000);
                            if let Ok(resps) = client.fetch_messages_for_partition(req) {
                                let mut fwd_msgs = Vec::<ForwardMode>::new();
                                resps.iter().for_each(|resp| {
                                    for t in resp.topics() {
                                        for p in t.partitions() {
                                            if let Ok(data) = p.data() {
                                                let msg = (0..data.messages().len())
                                                    .filter_map(|index| {
                                                        let msg = data.messages().get(index)?;
                                                        rmp_serde::from_slice::<ForwardMode>(
                                                            msg.value,
                                                        )
                                                        .ok()
                                                    })
                                                    .collect::<Vec<_>>();
                                                fwd_msgs.extend(msg);
                                            }
                                        }
                                    }
                                });
                                Some(fwd_msgs)
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .flat_map(|msg| {
                            msg.entries.into_iter().filter_map(|entry| {
                                let raw = entry.record.get("message")?;
                                Some((entry.time, raw.clone()))
                            })
                        })
                        .collect::<HashMap<_, _>>();
                    let events = message_ids
                        .iter()
                        .filter_map(|message_id| {
                            let raw = entries.get(message_id)?;
                            let message_id = bigdecimal::FromPrimitive::from_u64(*message_id)?;
                            let raw_event = Some(bytes_to_string(raw));

                            Some(Event {
                                message_id,
                                raw_event,
                                data_source_id,
                            })
                        })
                        .collect::<Vec<_>>();

                    let api_url = format!("http://{}/api/event", reviewd_addr);
                    let _ = send_http_put_request(&events, &api_url).await;
                }
            }
        }
        Err(e) => log::error!("Failed to fetch events with no raw_event: {}", e),
    }

    Ok(())
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

pub(crate) async fn send_http_get_request(api_url: &str) -> Result<String, reqwest::Error> {
    reqwest::Client::new()
        .get(api_url)
        .send()
        .await?
        .text()
        .await
}

async fn send_http_put_request<T>(data: &[T], api_url: &str) -> Result<(), ()>
where
    T: 'static + Clone + Send + Serialize + Sync,
{
    let client = reqwest::Client::new();
    let reqs = data
        .chunks(40)
        .map(|chunk| client.put(api_url).json(chunk).send())
        .collect::<Vec<_>>();

    if futures::future::join_all(reqs)
        .await
        .into_iter()
        .map(|r| r.and_then(reqwest::Response::error_for_status))
        .any(|r| r.is_err())
    {
        Err(())
    } else {
        Ok(())
    }
}
