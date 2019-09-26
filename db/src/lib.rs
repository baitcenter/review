#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use chrono::{NaiveDateTime, Utc};
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use futures::future;
use futures::prelude::*;
use models::*;
use remake::event::Identifiable;
use remake::stream::EventorKafka;
use std::collections::HashMap;

pub mod error;
pub use self::error::{DatabaseError, Error, ErrorKind};
pub mod models;
mod schema;

embed_migrations!();

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
type ConnType = PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;
pub type ClusterResponse = (
    Option<String>,        // cluster_id
    Option<i32>,           // detector_id
    Option<String>,        // qualifier
    Option<String>,        // status
    Option<String>,        // category
    Option<String>,        // signature
    Option<String>,        // data_source
    Option<usize>,         // size
    Option<f64>,           // score
    Option<Example>,       // examples
    Option<NaiveDateTime>, // last_modification_time
);
pub type SelectCluster = (
    bool, // cluster_id,
    bool, // detector_id
    bool, // qualifier
    bool, // status
    bool, // category
    bool, // signature
    bool, // data_source
    bool, // size
    bool, // score
    bool, // examples
    bool, // last_modification_time
);

#[derive(Clone)]
pub struct DB {
    pub pool: Pool,
    pub kafka_url: String,
}

impl DB {
    pub fn new(database_url: &str, kafka_url: String) -> impl Future<Item = Self, Error = Error> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let db = match Pool::new(manager) {
            Ok(pool) => match pool.get() {
                Ok(conn) => embedded_migrations::run(&conn)
                    .map(|()| Self { pool, kafka_url })
                    .map_err(Into::into),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        };

        future::result(db)
    }

    fn get_connection(&self) -> Result<ConnType, Error> {
        self.pool.get().map_err(Into::into)
    }

    pub fn add_category(&self, new_category: &str) -> impl Future<Item = usize, Error = Error> {
        use schema::category::dsl::*;
        let insert_result = self.get_connection().and_then(|conn| {
            diesel::insert_into(category)
                .values((name.eq(new_category),))
                .execute(&conn)
                .map_err(Into::into)
        });

        future::result(insert_result)
    }

    fn add_data_source(&self, data_source: &str, data_type: &str) -> i32 {
        use schema::data_source::dsl;

        let _: Result<usize, Error> = self.get_connection().and_then(|conn| {
            diesel::insert_into(dsl::data_source)
                .values((
                    dsl::topic_name.eq(data_source),
                    dsl::data_type.eq(data_type),
                ))
                .on_conflict(dsl::topic_name)
                .do_nothing()
                .execute(&conn)
                .map_err(Into::into)
        });

        DB::get_data_source_id(self, data_source).unwrap_or_default()
    }

    pub fn add_raw_events(
        &self,
        data_source: &str,
        max_event_count: usize,
    ) -> impl Future<Item = usize, Error = Error> {
        if let (
            Ok(event_ids_from_clusters),
            Ok(event_ids_from_outliers),
            Ok(consumer),
            Ok(data_source_id),
        ) = (
            DB::get_event_ids_from_cluster(self, data_source),
            DB::get_event_ids_from_outlier(self, data_source),
            EventorKafka::new(&self.kafka_url, data_source, "REviewd"),
            DB::get_data_source_id(self, data_source),
        ) {
            use schema::*;
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
            let mut update_lists = Vec::<UpdateRawEventId>::new();
            let raw_events: Vec<InsertRawEvent> =
                EventorKafka::fetch_messages(&consumer, max_event_count)
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
                let execution_result = self
                    .get_connection()
                    .and_then(|conn| {
                        Ok((
                            diesel::insert_into(raw_event::dsl::raw_event)
                                .values(&raw_events)
                                .execute(&*conn)
                                .map_err(Into::into),
                            conn,
                        ))
                    })
                    .and_then(|(row, conn)| {
                        if let Ok(raw_events) =
                            DB::get_raw_events_by_data_source_id(self, data_source_id)
                        {
                            let raw_events = raw_events
                                .into_iter()
                                .map(|e| (e.1, e.0))
                                .collect::<HashMap<Vec<u8>, i32>>();
                            for u in update_lists {
                                if let Some(raw_event_id) = raw_events.get(&u.data) {
                                    if u.is_cluster {
                                        use schema::clusters::dsl;
                                        let _ =
                                            diesel::update(dsl::clusters.filter(dsl::id.eq(u.id)))
                                                .set(dsl::raw_event_id.eq(Some(raw_event_id)))
                                                .execute(&*conn);
                                    } else {
                                        use schema::outliers::dsl;
                                        let _ =
                                            diesel::update(dsl::outliers.filter(dsl::id.eq(u.id)))
                                                .set(dsl::raw_event_id.eq(Some(raw_event_id)))
                                                .execute(&*conn);
                                    }
                                }
                            }
                        }
                        row
                    });
                return future::result(execution_result);
            }
        }

        future::result(Ok(0))
    }

    fn get_event_ids_from_cluster(&self, data_source: &str) -> Result<Vec<(i32, u64)>, Error> {
        use schema::clusters::dsl;
        if let Ok(data_source_id) = DB::get_data_source_id(self, data_source) {
            self.get_connection()
                .and_then(|conn| {
                    dsl::clusters
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

    fn get_event_ids_from_outlier(&self, data_source: &str) -> Result<Vec<(i32, u64)>, Error> {
        use schema::outliers::dsl;
        if let Ok(data_source_id) = DB::get_data_source_id(self, data_source) {
            self.get_connection()
                .and_then(|conn| {
                    dsl::outliers
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

    fn get_raw_events_by_data_source_id(
        &self,
        data_source_id: i32,
    ) -> Result<Vec<(i32, Vec<u8>)>, Error> {
        use schema::raw_event::dsl;
        self.get_connection().and_then(|conn| {
            dsl::raw_event
                .filter(dsl::data_source_id.eq(data_source_id))
                .select((dsl::raw_event_id, dsl::data))
                .load::<(i32, Vec<u8>)>(&conn)
                .map_err(Into::into)
        })
    }

    fn get_raw_event_by_raw_event_id(&self, raw_event_id: i32) -> Result<Vec<u8>, Error> {
        use schema::raw_event::dsl;
        self.get_connection().and_then(|conn| {
            dsl::raw_event
                .filter(dsl::raw_event_id.eq(raw_event_id))
                .select(dsl::data)
                .first::<Vec<u8>>(&conn)
                .map_err(Into::into)
        })
    }

    pub fn get_category_table(&self) -> impl Future<Item = Vec<CategoryTable>, Error = Error> {
        let category_table = self.get_connection().and_then(|conn| {
            schema::category::dsl::category
                .load::<CategoryTable>(&conn)
                .map_err(Into::into)
        });

        future::result(category_table)
    }

    pub fn get_cluster_table(&self) -> impl Future<Item = Vec<ClusterResponse>, Error = Error> {
        let cluster_table = self
            .get_connection()
            .and_then(|conn| {
                schema::clusters::dsl::clusters
                    .inner_join(schema::status::dsl::status)
                    .inner_join(schema::qualifier::dsl::qualifier)
                    .inner_join(schema::category::dsl::category)
                    .inner_join(schema::data_source::dsl::data_source)
                    .load::<(
                        ClustersTable,
                        StatusTable,
                        QualifierTable,
                        CategoryTable,
                        DataSourceTable,
                    )>(&conn)
                    .map_err(Into::into)
            })
            .and_then(|data| {
                let clusters = data
                    .into_iter()
                    .map(|d| {
                        let event_ids = if let Some(event_ids) =
                            d.0.event_ids
                                .and_then(|eg| rmp_serde::decode::from_slice::<Vec<u64>>(&eg).ok())
                        {
                            let raw_event = if let Some(raw_event_id) = d.0.raw_event_id {
                                DB::get_raw_event_by_raw_event_id(self, raw_event_id)
                                    .ok()
                                    .map_or("-".to_string(), |raw_events| {
                                        bytes_to_string(&raw_events)
                                    })
                            } else {
                                "-".to_string()
                            };
                            Some(Example {
                                raw_event,
                                event_ids,
                            })
                        } else {
                            None
                        };
                        let cluster_size = d.0.size.parse::<usize>().unwrap_or(0);
                        (
                            d.0.cluster_id,
                            Some(d.0.detector_id),
                            Some(d.2.description),
                            Some(d.1.description),
                            Some(d.3.name),
                            Some(d.0.signature),
                            Some(d.4.topic_name),
                            Some(cluster_size),
                            d.0.score,
                            event_ids,
                            d.0.last_modification_time,
                        )
                    })
                    .collect();
                Ok(clusters)
            });

        future::result(cluster_table)
    }

    pub fn get_qualifier_table(&self) -> impl Future<Item = Vec<QualifierTable>, Error = Error> {
        let qualifier_table = self.get_connection().and_then(|conn| {
            schema::qualifier::dsl::qualifier
                .load::<QualifierTable>(&conn)
                .map_err(Into::into)
        });

        future::result(qualifier_table)
    }

    pub fn get_status_table(&self) -> impl Future<Item = Vec<StatusTable>, Error = Error> {
        let status_table = self.get_connection().and_then(|conn| {
            schema::status::dsl::status
                .load::<StatusTable>(&conn)
                .map_err(Into::into)
        });

        future::result(status_table)
    }

    pub fn get_outliers_table(
        &self,
    ) -> impl Future<Item = Vec<(OutliersTable, DataSourceTable)>, Error = Error> {
        let outliers_table = self.get_connection().and_then(|conn| {
            schema::outliers::dsl::outliers
                .inner_join(schema::data_source::dsl::data_source)
                .load::<(OutliersTable, DataSourceTable)>(&conn)
                .map_err(Into::into)
        });

        future::result(outliers_table)
    }

    pub fn execute_select_cluster_query(
        &self,
        where_clause: Option<String>,
        limit: Option<i64>,
        select: SelectCluster,
    ) -> impl Future<Item = (Vec<ClusterResponse>, bool), Error = Error> {
        let cluster = self.get_connection()
            .and_then(|conn| {
                let mut query = "SELECT * FROM clusters INNER JOIN category ON clusters.category_id = category.category_id INNER JOIN qualifier ON clusters.qualifier_id = qualifier.qualifier_id INNER JOIN status ON clusters.status_id = status.status_id INNER JOIN data_source ON clusters.data_source_id = data_source.data_source_id".to_string();
                if let Some(where_clause) = where_clause {
                    query.push_str(&format!(" WHERE {}", where_clause));
                }
                if let Some(limit) = limit {
                    query.push_str(&format!(" LIMIT {}", limit));
                }
                query.push_str(";");
                diesel::sql_query(query)
                    .load::<(ClustersTable, StatusTable, QualifierTable, CategoryTable, DataSourceTable)>(&conn)
                    .map_err(Into::into)
            })
            .and_then(|data| {
                let clusters: Vec<ClusterResponse> = data
                    .into_iter()
                    .map(|d| {
                        let cluster_id = if select.0 { d.0.cluster_id } else { None };
                        let detector_id = if select.1 {
                            Some(d.0.detector_id)
                        } else {
                            None
                        };
                        let qualifier = if select.2 { Some(d.2.description) } else { None };
                        let status = if select.3 { Some(d.1.description) } else { None };
                        let category = if select.4 { Some(d.3.name) } else { None };
                        let signature = if select.5 { Some(d.0.signature) } else { None };
                        let data_source = if select.6 {
                            Some(d.4.topic_name.clone())
                        } else {
                            None
                        };
                        let cluster_size = if select.7 {
                            Some(d.0.size.parse::<usize>().unwrap_or(0))
                        } else {
                            None
                        };
                        let score = if select.8 { d.0.score } else { None };
                        let event_ids = if select.9 {
                            if let Some(event_ids) = d.0.event_ids.and_then(|eg| rmp_serde::decode::from_slice::<Vec<u64>>(&eg).ok()) {
                                let raw_event = if let Some(raw_event_id) = d.0.raw_event_id {
                                    DB::get_raw_event_by_raw_event_id(self, raw_event_id)
                                        .ok()
                                        .map_or("-".to_string(), |raw_events| bytes_to_string(&raw_events))
                                } else {
                                    "-".to_string()
                                };
                                Some(Example {raw_event, event_ids})
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let time = if select.10 {
                            d.0.last_modification_time
                        } else {
                            None
                        };
                        (
                            cluster_id,
                            detector_id,
                            qualifier,
                            status,
                            category,
                            signature,
                            data_source,
                            cluster_size,
                            score,
                            event_ids,
                            time,
                        )
                    })
                    .collect();
                Ok((clusters, select.8))
            });

        future::result(cluster)
    }

    pub fn execute_select_outlier_query(
        &self,
        data_source: &str,
    ) -> impl Future<Item = Vec<(OutliersTable, DataSourceTable)>, Error = Error> {
        use schema::outliers::dsl;
        let result = self.get_connection().and_then(|conn| {
            if let Ok(data_source_id) = DB::get_data_source_id(self, data_source) {
                dsl::outliers
                    .inner_join(schema::data_source::dsl::data_source)
                    .filter(dsl::data_source_id.eq(data_source_id))
                    .load::<(OutliersTable, DataSourceTable)>(&conn)
                    .map_err(Into::into)
            } else {
                Ok(Vec::<(OutliersTable, DataSourceTable)>::new())
            }
        });

        future::result(result)
    }

    pub fn add_outliers(
        &self,
        new_outliers: &[OutlierUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::outliers::dsl;
        let insert_result = self.get_connection().and_then(|conn| {
            let insert_outliers: Vec<_> = new_outliers
                .iter()
                .filter_map(|new_outlier| {
                    let o_size = Some(new_outlier.event_ids.len().to_string());
                    let event_ids =
                        rmp_serde::encode::to_vec(&new_outlier.event_ids).unwrap_or_default();
                    DB::get_data_source_id(self, &new_outlier.data_source)
                        .ok()
                        .map(|data_source_id| {
                            (
                                dsl::raw_event.eq(new_outlier.outlier.to_vec()),
                                dsl::data_source_id.eq(data_source_id),
                                dsl::event_ids.eq(event_ids),
                                dsl::raw_event_id.eq(Option::<i32>::None),
                                dsl::size.eq(o_size),
                            )
                        })
                })
                .collect();

            if !insert_outliers.is_empty() {
                diesel::insert_into(schema::outliers::dsl::outliers)
                    .values(&insert_outliers)
                    .execute(&*conn)
                    .map_err(Into::into)
            } else {
                Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
            }
        });
        future::result(insert_result)
    }

    pub fn delete_outliers(
        &self,
        outliers: &[String],
        data_source: &str,
    ) -> future::FutureResult<(), Error> {
        use schema::clusters::dsl as c_dsl;
        use schema::outliers::dsl as o_dsl;
        if let (Ok(data_source_id), Ok(conn)) = (
            DB::get_data_source_id(self, data_source),
            self.get_connection(),
        ) {
            let outliers_from_database = o_dsl::outliers
                .filter(o_dsl::data_source_id.eq(data_source_id))
                .load::<OutliersTable>(&conn)
                .unwrap_or_default();

            let deleted_outliers = outliers
                .iter()
                .filter_map(|outlier| {
                    let _ = diesel::delete(
                        o_dsl::outliers.filter(
                            o_dsl::data_source_id
                                .eq(data_source_id)
                                .and(o_dsl::raw_event.eq(outlier.as_bytes())),
                        ),
                    )
                    .execute(&conn);

                    if let Some(o) = outliers_from_database
                        .iter()
                        .find(|o| o.raw_event == outlier.as_bytes())
                    {
                        if let (event_ids, Some(raw_event_id)) =
                            (o.event_ids.as_ref(), o.raw_event_id)
                        {
                            if let Ok(event_ids) =
                                rmp_serde::decode::from_slice::<Vec<u64>>(event_ids)
                            {
                                return Some((event_ids, raw_event_id));
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();

            let clusters_from_database = c_dsl::clusters
                .select((c_dsl::id, c_dsl::event_ids))
                .filter(
                    c_dsl::data_source_id
                        .eq(data_source_id)
                        .and(c_dsl::raw_event_id.is_null()),
                )
                .load::<(i32, Option<Vec<u8>>)>(&conn)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(id, event_ids)| {
                    if let (id, Some(event_ids)) = (id, event_ids) {
                        if let Ok(event_ids) = rmp_serde::decode::from_slice::<Vec<u64>>(&event_ids)
                        {
                            return Some((id, event_ids));
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();

            let update_clusters = deleted_outliers
                .into_iter()
                .filter_map(|(event_ids, raw_event_id)| {
                    event_ids
                        .iter()
                        .find_map(|e| {
                            clusters_from_database.iter().find_map(
                                |(id, event_ids_from_database)| {
                                    if event_ids_from_database.contains(e) {
                                        Some(id)
                                    } else {
                                        None
                                    }
                                },
                            )
                        })
                        .map(|id| (id, raw_event_id))
                })
                .collect::<HashMap<_, _>>();

            for (id, raw_event_id) in update_clusters {
                let _ = diesel::update(c_dsl::clusters.filter(c_dsl::id.eq(id)))
                    .set(c_dsl::raw_event_id.eq(raw_event_id))
                    .execute(&conn);
            }
        }

        future::result(Ok(()))
    }

    pub fn update_outliers(
        &self,
        outlier_update: &[OutlierUpdate],
    ) -> future::FutureResult<usize, Error> {
        use schema::outliers::dsl;
        let mut query = dsl::outliers.into_boxed();
        for outlier in outlier_update.iter() {
            if let Ok(data_source_id) = DB::get_data_source_id(self, &outlier.data_source) {
                query = query.or_filter(
                    dsl::raw_event
                        .eq(&outlier.outlier)
                        .and(dsl::data_source_id.eq(data_source_id)),
                );
            }
        }
        let execution_result = self.get_connection().and_then(|conn| {
            query
                .load::<OutliersTable>(&conn)
                .map_err(Into::into)
                .and_then(|outlier_list| {
                    let replace_outliers: Vec<_> = outlier_update
                        .iter()
                        .filter_map(|o| {
                            if let Some(outlier) = outlier_list
                                .iter()
                                .find(|outlier| o.outlier == outlier.raw_event)
                            {
                                let new_size = o.event_ids.len();
                                let o_size = match &outlier.size {
                                    Some(current_size) => {
                                        if let Ok(current_size) = current_size.parse::<usize>() {
                                            // check if sum of new_size and current_size exceeds max_value
                                            // if it does, we cannot calculate sum anymore, so reset the value of size
                                            if new_size > usize::max_value() - current_size {
                                                new_size.to_string()
                                            } else {
                                                (current_size + new_size).to_string()
                                            }
                                        } else {
                                            new_size.to_string()
                                        }
                                    }
                                    None => new_size.to_string(),
                                };
                                let mut event_ids =
                                    rmp_serde::decode::from_slice::<Vec<u64>>(&outlier.event_ids)
                                        .unwrap_or_default();
                                event_ids.extend(&o.event_ids);
                                // only store most recent 25 event_ids per outlier
                                let event_ids = if event_ids.len() > 25 {
                                    event_ids.sort();
                                    let (_, event_ids) = event_ids.split_at(event_ids.len() - 25);
                                    event_ids
                                } else {
                                    &event_ids
                                };
                                let event_ids =
                                    rmp_serde::encode::to_vec(event_ids).unwrap_or_default();
                                let data_source_id = DB::get_data_source_id(self, &o.data_source)
                                    .unwrap_or_default();
                                if data_source_id != 0 {
                                    Some((
                                        dsl::raw_event.eq(o.outlier.clone()),
                                        dsl::data_source_id.eq(data_source_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(outlier.raw_event_id),
                                        dsl::size.eq(Some(o_size)),
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                // only store most recent 25 event_ids per outlier
                                let mut event_ids_cloned = o.event_ids.clone();
                                let event_ids = if event_ids_cloned.len() > 25 {
                                    event_ids_cloned.sort();
                                    let (_, event_ids) =
                                        event_ids_cloned.split_at(o.event_ids.len() - 25);
                                    event_ids
                                } else {
                                    &o.event_ids
                                };
                                let size = o.event_ids.len();
                                let event_ids =
                                    rmp_serde::encode::to_vec(event_ids).unwrap_or_default();
                                let data_source_id = DB::get_data_source_id(self, &o.data_source)
                                    .unwrap_or_else(|_| {
                                        DB::add_data_source(
                                            self,
                                            &o.data_source,
                                            &o.data_source_type,
                                        )
                                    });
                                if data_source_id != 0 {
                                    Some((
                                        dsl::raw_event.eq(o.outlier.clone()),
                                        dsl::data_source_id.eq(data_source_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(Option::<i32>::None),
                                        dsl::size.eq(Some(size.to_string())),
                                    ))
                                } else {
                                    None
                                }
                            }
                        })
                        .collect();

                    if !replace_outliers.is_empty() {
                        diesel::insert_into(dsl::outliers)
                            .values(&replace_outliers)
                            .on_conflict((dsl::raw_event, dsl::data_source_id))
                            .do_update()
                            .set((
                                dsl::id.eq(excluded(dsl::id)),
                                dsl::event_ids.eq(excluded(dsl::event_ids)),
                                dsl::raw_event_id.eq(excluded(dsl::raw_event_id)),
                                dsl::size.eq(excluded(dsl::size)),
                            ))
                            .execute(&*conn)
                            .map_err(Into::into)
                    } else {
                        Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
                    }
                })
        });

        future::result(execution_result)
    }

    pub fn update_qualifiers(
        &self,
        qualifier_update: &[QualifierUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::clusters::dsl;
        let result = self.get_connection().and_then(|conn| {
            let timestamp = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
            let status_id = match DB::get_status_id(self, "reviewed") {
                Ok(id) => id,
                _ => 1,
            };
            let row = qualifier_update
                .iter()
                .map(|q| {
                    if let (Ok(qualifier_id), Ok(data_source_id)) = (
                        DB::get_qualifier_id(&self, &q.qualifier),
                        DB::get_data_source_id(self, &q.data_source),
                    ) {
                        let target = dsl::clusters.filter(
                            dsl::cluster_id
                                .eq(&q.cluster_id)
                                .and(dsl::data_source_id.eq(data_source_id)),
                        );
                        diesel::update(target)
                            .set((
                                dsl::qualifier_id.eq(qualifier_id),
                                dsl::status_id.eq(status_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else {
                        Err(Error::from(ErrorKind::DatabaseTransactionError(
                            DatabaseError::RecordNotExist,
                        )))
                    }
                })
                .filter_map(Result::ok)
                .collect::<Vec<usize>>()
                .iter()
                .sum();

            if row == 0 {
                Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
            } else {
                Ok(row)
            }
        });

        future::result(result)
    }

    fn get_category_id(&self, category: &str) -> Result<i32, Error> {
        use schema::category::dsl;
        self.get_connection().and_then(|conn| {
            dsl::category
                .select(dsl::category_id)
                .filter(dsl::name.eq(category))
                .first::<i32>(&conn)
                .map_err(Into::into)
        })
    }

    fn get_data_source_id(&self, data_source: &str) -> Result<i32, Error> {
        use schema::data_source::dsl;
        self.get_connection().and_then(|conn| {
            dsl::data_source
                .select(dsl::data_source_id)
                .filter(dsl::topic_name.eq(data_source))
                .first::<i32>(&conn)
                .map_err(Into::into)
        })
    }

    fn get_qualifier_id(&self, qualifier: &str) -> Result<i32, Error> {
        use schema::qualifier::dsl;
        self.get_connection().and_then(|conn| {
            dsl::qualifier
                .select(dsl::qualifier_id)
                .filter(dsl::description.eq(qualifier))
                .first::<i32>(&conn)
                .map_err(Into::into)
        })
    }

    fn get_status_id(&self, status: &str) -> Result<i32, Error> {
        use schema::status::dsl;
        self.get_connection().and_then(|conn| {
            dsl::status
                .select(dsl::status_id)
                .filter(dsl::description.eq(status))
                .first::<i32>(&conn)
                .map_err(Into::into)
        })
    }

    pub fn update_cluster(
        &self,
        cluster_id: &str,
        data_source: &str,
        new_cluster_id: Option<String>,
        category: Option<String>,
        qualifier: Option<String>,
    ) -> future::FutureResult<(u8, bool, String), Error> {
        use schema::clusters::dsl;

        if let Ok(data_source_id) = DB::get_data_source_id(self, &data_source) {
            let query = diesel::update(dsl::clusters).filter(
                dsl::cluster_id
                    .eq(cluster_id)
                    .and(dsl::data_source_id.eq(data_source_id)),
            );
            let timestamp = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
            let category_id =
                category.and_then(|category| DB::get_category_id(self, &category).ok());
            let (qualifier_id, is_benign) = qualifier.map_or((None, false), |qualifier| {
                (
                    DB::get_qualifier_id(self, &qualifier).ok(),
                    qualifier == "benign",
                )
            });
            let status_id = match DB::get_status_id(self, "reviewed") {
                Ok(id) => id,
                _ => 1,
            };
            let execution_result = self
                .get_connection()
                .and_then(|conn| {
                    if let (Some(cluster_id), Some(category_id), Some(qualifier_id)) =
                        (&new_cluster_id, &category_id, &qualifier_id)
                    {
                        query
                            .set((
                                dsl::cluster_id.eq(cluster_id),
                                dsl::category_id.eq(category_id),
                                dsl::qualifier_id.eq(qualifier_id),
                                dsl::status_id.eq(status_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let (Some(cluster_id), Some(category_id)) =
                        (&new_cluster_id, &category_id)
                    {
                        query
                            .set((
                                dsl::cluster_id.eq(cluster_id),
                                dsl::category_id.eq(category_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let (Some(cluster_id), Some(qualifier_id)) =
                        (&new_cluster_id, &qualifier_id)
                    {
                        query
                            .set((
                                dsl::cluster_id.eq(cluster_id),
                                dsl::qualifier_id.eq(qualifier_id),
                                dsl::status_id.eq(status_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let (Some(category_id), Some(qualifier_id)) =
                        (&category_id, &qualifier_id)
                    {
                        query
                            .set((
                                dsl::category_id.eq(category_id),
                                dsl::qualifier_id.eq(qualifier_id),
                                dsl::status_id.eq(status_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let Some(cluster_id) = &new_cluster_id {
                        query
                            .set((
                                dsl::cluster_id.eq(cluster_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let Some(category_id) = &category_id {
                        query
                            .set((
                                dsl::category_id.eq(category_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else if let Some(qualifier_id) = &qualifier_id {
                        query
                            .set((
                                dsl::qualifier_id.eq(qualifier_id),
                                dsl::status_id.eq(status_id),
                                dsl::last_modification_time.eq(timestamp),
                            ))
                            .execute(&conn)
                            .map_err(Into::into)
                    } else {
                        Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
                    }
                })
                .and_then(|row| Ok((row as u8, is_benign, data_source.to_string())));

            future::result(execution_result)
        } else {
            future::result(Err(ErrorKind::DatabaseTransactionError(
                DatabaseError::Other,
            )
            .into()))
        }
    }

    pub fn update_clusters(
        &self,
        cluster_update: &[ClusterUpdate],
    ) -> future::FutureResult<usize, Error> {
        use schema::clusters::dsl;
        use serde::Serialize;
        #[derive(Debug, Queryable, Serialize)]
        pub struct Cluster {
            cluster_id: Option<String>,
            signature: String,
            event_ids: Option<Vec<u8>>,
            raw_event_id: Option<i32>,
            size: String,
            category_id: i32,
            qualifier_id: i32,
            status_id: i32,
        }
        let mut query = dsl::clusters.into_boxed();
        for cluster in cluster_update.iter() {
            if let Ok(data_source_id) = DB::get_data_source_id(self, &cluster.data_source) {
                query = query.or_filter(
                    dsl::cluster_id
                        .eq(&cluster.cluster_id)
                        .and(dsl::data_source_id.eq(data_source_id)),
                );
            }
        }
        let execution_result = self.get_connection().and_then(|conn| {
            query
                .select((
                    dsl::cluster_id,
                    dsl::signature,
                    dsl::event_ids,
                    dsl::raw_event_id,
                    dsl::size,
                    dsl::category_id,
                    dsl::qualifier_id,
                    dsl::status_id,
                ))
                .load::<Cluster>(&conn)
                .map_err(Into::into)
                .and_then(|cluster_list| {
                    let replace_clusters: Vec<_> = cluster_update
                        .iter()
                        .filter_map(|c| {
                            if let Some(cluster) = cluster_list
                                .iter()
                                .find(|cluster| Some(c.cluster_id.clone()) == cluster.cluster_id)
                            {
                                c.event_ids.as_ref()?;
                                let now = Utc::now();
                                let timestamp = NaiveDateTime::from_timestamp(now.timestamp(), 0);

                                let sig = match &c.signature {
                                    Some(sig) => sig.clone(),
                                    None => cluster.signature.clone(),
                                };
                                let event_ids = DB::merge_cluster_examples(
                                    cluster.event_ids.clone(),
                                    c.event_ids.clone(),
                                );
                                let cluster_size = match c.size {
                                    Some(new_size) => {
                                        if let Ok(current_size) =
                                            cluster.size.clone().parse::<usize>()
                                        {
                                            // check if sum of new_size and current_size exceeds max_value
                                            // if it does, we cannot calculate sum anymore, so reset the value of size
                                            if new_size > usize::max_value() - current_size {
                                                new_size.to_string()
                                            } else {
                                                (current_size + new_size).to_string()
                                            }
                                        } else {
                                            cluster.size.clone()
                                        }
                                    }
                                    None => cluster.size.clone(),
                                };
                                let data_source_id = DB::get_data_source_id(self, &c.data_source)
                                    .unwrap_or_default();
                                if data_source_id != 0 {
                                    Some((
                                        dsl::cluster_id.eq(c.cluster_id.clone()),
                                        dsl::category_id.eq(cluster.category_id),
                                        dsl::detector_id.eq(c.detector_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(cluster.raw_event_id),
                                        dsl::qualifier_id.eq(cluster.qualifier_id),
                                        dsl::status_id.eq(cluster.status_id),
                                        dsl::signature.eq(sig),
                                        dsl::size.eq(cluster_size),
                                        dsl::score.eq(c.score),
                                        dsl::data_source_id.eq(data_source_id),
                                        dsl::last_modification_time.eq(Some(timestamp)),
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                let event_ids = match &c.event_ids {
                                    Some(eg) => rmp_serde::encode::to_vec(&eg).ok(),
                                    None => None,
                                };
                                let sig = match &c.signature {
                                    Some(sig) => sig.clone(),
                                    None => "-".to_string(),
                                };
                                let cluster_size = match c.size {
                                    Some(cluster_size) => cluster_size.to_string(),
                                    None => "1".to_string(),
                                };
                                let data_source_id = DB::get_data_source_id(self, &c.data_source)
                                    .unwrap_or_else(|_| {
                                        DB::add_data_source(
                                            self,
                                            &c.data_source,
                                            &c.data_source_type,
                                        )
                                    });
                                if data_source_id != 0 {
                                    Some((
                                        dsl::cluster_id.eq(c.cluster_id.clone()),
                                        dsl::category_id.eq(1),
                                        dsl::detector_id.eq(c.detector_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(None),
                                        dsl::qualifier_id.eq(2),
                                        dsl::status_id.eq(2),
                                        dsl::signature.eq(sig),
                                        dsl::size.eq(cluster_size),
                                        dsl::score.eq(c.score),
                                        dsl::data_source_id.eq(data_source_id),
                                        dsl::last_modification_time.eq(None),
                                    ))
                                } else {
                                    None
                                }
                            }
                        })
                        .collect();

                    if !replace_clusters.is_empty() {
                        diesel::insert_into(dsl::clusters)
                            .values(&replace_clusters)
                            .on_conflict((dsl::cluster_id, dsl::data_source_id))
                            .do_update()
                            .set((
                                dsl::id.eq(excluded(dsl::id)),
                                dsl::category_id.eq(excluded(dsl::category_id)),
                                dsl::detector_id.eq(excluded(dsl::detector_id)),
                                dsl::event_ids.eq(excluded(dsl::event_ids)),
                                dsl::raw_event_id.eq(excluded(dsl::raw_event_id)),
                                dsl::qualifier_id.eq(excluded(dsl::qualifier_id)),
                                dsl::status_id.eq(excluded(dsl::status_id)),
                                dsl::signature.eq(excluded(dsl::signature)),
                                dsl::size.eq(excluded(dsl::size)),
                                dsl::score.eq(excluded(dsl::score)),
                                dsl::last_modification_time
                                    .eq(excluded(dsl::last_modification_time)),
                            ))
                            .execute(&*conn)
                            .map_err(Into::into)
                    } else {
                        Ok(0)
                    }
                })
        });

        future::result(execution_result)
    }

    pub fn add_clusters(
        &self,
        new_clusters: &[ClusterUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::clusters::dsl;
        let insert_result = self.get_connection().and_then(|conn| {
            let insert_clusters: Vec<_> = new_clusters
                .iter()
                .filter_map(|c| {
                    let event_ids = match &c.event_ids {
                        Some(eg) => rmp_serde::encode::to_vec(&eg).ok(),
                        None => None,
                    };

                    // Signature is required field in central repo database
                    // but if new cluster information does not have signature field,
                    // we use '-' as a signature
                    let sig = match &c.signature {
                        Some(sig) => sig.clone(),
                        None => "-".to_string(),
                    };
                    let cluster_size = match c.size {
                        Some(cluster_size) => cluster_size.to_string(),
                        None => "1".to_string(),
                    };
                    let data_source_id = DB::get_data_source_id(self, &c.data_source)
                        .unwrap_or_else(|_| {
                            DB::add_data_source(self, &c.data_source, &c.data_source_type)
                        });
                    if data_source_id != 0 {
                        // We always insert 1 for category_id
                        // "unknown" for qualifier_id, and "pending review" for status_id.
                        Some((
                            dsl::cluster_id.eq(Some(c.cluster_id.to_string())),
                            dsl::category_id.eq(1),
                            dsl::detector_id.eq(c.detector_id),
                            dsl::event_ids.eq(event_ids),
                            dsl::raw_event_id.eq(Option::<i32>::None),
                            dsl::qualifier_id.eq(2),
                            dsl::status_id.eq(2),
                            dsl::signature.eq(sig),
                            dsl::size.eq(cluster_size),
                            dsl::score.eq(c.score),
                            dsl::data_source_id.eq(data_source_id),
                            dsl::last_modification_time.eq(Option::<NaiveDateTime>::None),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            if !insert_clusters.is_empty() {
                diesel::insert_into(schema::clusters::dsl::clusters)
                    .values(&insert_clusters)
                    .execute(&*conn)
                    .map_err(Into::into)
            } else {
                Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
            }
        });

        future::result(insert_result)
    }

    fn merge_cluster_examples(
        current_examples: Option<Vec<u8>>,
        new_examples: Option<Vec<u64>>,
    ) -> Option<Vec<u8>> {
        let max_example_num: usize = 25;
        new_examples.map_or(current_examples.clone(), |new_examples| {
            if new_examples.len() >= max_example_num {
                match rmp_serde::encode::to_vec(&new_examples) {
                    Ok(new_examples) => Some(new_examples),
                    Err(_) => current_examples,
                }
            } else {
                current_examples.map(|current_eg| {
                    match rmp_serde::decode::from_slice::<Vec<u64>>(&current_eg) {
                        Ok(mut eg) => {
                            eg.extend(new_examples);
                            let example = if eg.len() > max_example_num {
                                eg.sort();
                                let (_, eg) = eg.split_at(eg.len() - max_example_num);
                                rmp_serde::encode::to_vec(&eg)
                            } else {
                                rmp_serde::encode::to_vec(&eg)
                            };
                            example.unwrap_or(current_eg)
                        }
                        Err(_) => current_eg,
                    }
                })
            }
        })
    }

    pub fn update_category(
        &self,
        current_category: &str,
        new_category: &str,
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::category::dsl::*;
        let update_result = DB::get_category_id(&self, current_category).and_then(|_| {
            let target = category.filter(name.eq(current_category));
            self.get_connection().and_then(|conn| {
                diesel::update(target)
                    .set((name.eq(new_category),))
                    .execute(&conn)
                    .map_err(Into::into)
            })
        });
        future::result(update_result)
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}
