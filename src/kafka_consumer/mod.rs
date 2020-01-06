mod fetch;
use futures::future;
use log::error;
use serde::Deserialize;
use std::time::Duration;
use tokio::{task, time};

use crate::database::DataSource;

#[derive(Debug, Deserialize)]
pub(crate) struct KafkaConfig {
    kafka_url: String,
    reviewd_addr: String,
    interval: Option<u64>,
    max_offset_count: Option<usize>,
}
impl KafkaConfig {
    #[allow(clippy::must_use_candidate)]
    pub(crate) fn new(
        kafka_url: String,
        reviewd_addr: String,
        interval: Option<u64>,
        max_offset_count: Option<usize>,
    ) -> Self {
        Self {
            kafka_url,
            reviewd_addr,
            interval,
            max_offset_count,
        }
    }

    pub(crate) async fn periodically_fetch_kafka_message(self) -> Result<(), std::io::Error> {
        let mut interval =
            time::interval(Duration::from_secs(self.interval.unwrap_or_else(|| 900)));
        let max_offset_count: usize = self.max_offset_count.unwrap_or_else(|| 1000);
        let api_url = format!("http://{}/api/data_source", self.reviewd_addr);
        log::info!(
            "Starting periodic tasks with time interval {} second(s) and max_offset_count {}",
            self.interval.unwrap_or_else(|| 900),
            max_offset_count
        );
        loop {
            interval.tick().await;
            match fetch::send_http_get_request(&api_url).await {
                Ok(data_sources) => {
                    if let Ok(data_sources) = serde_json::from_str::<Vec<DataSource>>(&data_sources)
                    {
                        // Fetch unread kafka messages and insert metadata to the database
                        let tasks = data_sources.iter().map(|data_source| {
                            task::spawn(fetch::fetch_kafka_metadata(
                                self.kafka_url.clone(),
                                data_source.topic_name.clone(),
                                data_source.id,
                                self.reviewd_addr.clone(),
                                max_offset_count,
                            ))
                        });
                        future::join_all(tasks)
                            .await
                            .iter()
                            .for_each(|task| match task {
                                Err(e) => error!("{}", e),
                                Ok(Err(e)) => error!("{}", e),
                                _ => (),
                            });

                        // Fetch specific kafka messages using partition and offset
                        let tasks = data_sources.iter().map(|data_source| {
                            task::spawn(fetch::fetch_raw_events(
                                self.kafka_url.clone(),
                                data_source.topic_name.clone(),
                                data_source.id,
                                self.reviewd_addr.clone(),
                                max_offset_count,
                            ))
                        });
                        future::join_all(tasks)
                            .await
                            .iter()
                            .for_each(|task| match task {
                                Err(e) => error!("{}", e),
                                Ok(Err(e)) => error!("{}", e),
                                _ => (),
                            });
                    }
                }
                Err(e) => error!("Failed to fetch kafka topic names: {}", e),
            }
        }
    }
}
