#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use futures::future;
use futures::prelude::*;
use models::*;

pub mod error;
pub use self::error::{DatabaseError, Error, ErrorKind};
pub mod models;
mod schema;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
pub type ClusterResponse = (
    Option<String>,                // cluster_id
    Option<i32>,                   // detector_id
    Option<String>,                // qualifier
    Option<String>,                // status
    Option<String>,                // category
    Option<String>,                // signature
    Option<String>,                // data_source
    Option<usize>,                 // size
    Option<Vec<Example>>,          // examples
    Option<chrono::NaiveDateTime>, // last_modification_time
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
    bool, // examples
    bool, // last_modification_time
);

#[derive(Clone)]
pub struct DB {
    pub pool: Pool,
}

impl DB {
    pub fn new(database_url: &str) -> Box<Future<Item = Self, Error = Error> + Send + 'static> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let db = Pool::new(manager)
            .map(|pool| Self { pool })
            .map_err(Into::into);

        Box::new(future::result(db))
    }

    pub fn add_category(&self, new_category: &str) -> impl Future<Item = usize, Error = Error> {
        use schema::Category::dsl::*;
        let c = CategoryTable {
            category_id: None,
            category: new_category.to_string(),
        };
        let insert_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            diesel::insert_into(Category)
                .values(&c)
                .execute(&conn)
                .map_err(Into::into)
        });

        future::result(insert_result)
    }

    pub fn get_category_table(&self) -> impl Future<Item = Vec<CategoryTable>, Error = Error> {
        let category_table = self.pool.get().map_err(Into::into).and_then(|conn| {
            schema::Category::dsl::Category
                .load::<CategoryTable>(&conn)
                .map_err(Into::into)
        });

        future::result(category_table)
    }

    pub fn get_cluster_table(&self) -> impl Future<Item = Vec<ClusterResponse>, Error = Error> {
        let cluster_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| {
                schema::Clusters::dsl::Clusters
                    .inner_join(schema::Status::dsl::Status)
                    .inner_join(schema::Qualifier::dsl::Qualifier)
                    .inner_join(schema::Category::dsl::Category)
                    .load::<(ClustersTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                    .map_err(Into::into)
            })
            .and_then(|data| {
                let clusters = data
                    .into_iter()
                    .map(|d| {
                        let examples = d.0.examples.and_then(|eg| {
                            (rmp_serde::decode::from_slice(&eg)
                                as Result<Vec<Example>, rmp_serde::decode::Error>)
                                .ok()
                        });
                        let cluster_size = d.0.size.parse::<usize>().unwrap_or(0);
                        (
                            d.0.cluster_id,
                            Some(d.0.detector_id),
                            Some(d.2.qualifier),
                            Some(d.1.status),
                            Some(d.3.category),
                            Some(d.0.signature),
                            Some(d.0.data_source),
                            Some(cluster_size),
                            examples,
                            d.0.last_modification_time,
                        )
                    })
                    .collect();
                Ok(clusters)
            });

        future::result(cluster_table)
    }

    pub fn get_qualifier_table(&self) -> impl Future<Item = Vec<QualifierTable>, Error = Error> {
        let qualifier_table = self.pool.get().map_err(Into::into).and_then(|conn| {
            schema::Qualifier::dsl::Qualifier
                .load::<QualifierTable>(&conn)
                .map_err(Into::into)
        });

        future::result(qualifier_table)
    }

    pub fn get_status_table(&self) -> impl Future<Item = Vec<StatusTable>, Error = Error> {
        let status_table = self.pool.get().map_err(Into::into).and_then(|conn| {
            schema::Status::dsl::Status
                .load::<StatusTable>(&conn)
                .map_err(Into::into)
        });

        future::result(status_table)
    }

    pub fn get_outliers_table(&self) -> impl Future<Item = Vec<OutliersTable>, Error = Error> {
        let outliers_table = self.pool.get().map_err(Into::into).and_then(|conn| {
            schema::Outliers::dsl::Outliers
                .load::<OutliersTable>(&conn)
                .map_err(Into::into)
        });

        future::result(outliers_table)
    }

    pub fn execute_select_cluster_query(
        &self,
        where_clause: Option<String>,
        limit: Option<i64>,
        select: SelectCluster,
    ) -> impl Future<Item = Vec<ClusterResponse>, Error = Error> {
        let cluster = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| {
                let mut query = "SELECT * FROM Clusters INNER JOIN Category ON Clusters.category_id = Category.category_id INNER JOIN Qualifier ON Clusters.qualifier_id = Qualifier.qualifier_id INNER JOIN Status ON Clusters.status_id = Status.status_id".to_string();
                if let Some(where_clause) = where_clause {
                    query.push_str(&format!(" WHERE {}", where_clause));
                }
                if let Some(limit) = limit {
                    query.push_str(&format!(" LIMIT {}", limit));
                }
                query.push_str(";");
                diesel::sql_query(query)
                    .load::<(ClustersTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
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
                        let qualifier = if select.2 { Some(d.2.qualifier) } else { None };
                        let status = if select.3 { Some(d.1.status) } else { None };
                        let category = if select.4 { Some(d.3.category) } else { None };
                        let signature = if select.5 { Some(d.0.signature) } else { None };
                        let data_source = if select.6 {
                            Some(d.0.data_source)
                        } else {
                            None
                        };
                        let cluster_size = if select.7 {
                            Some(d.0.size.parse::<usize>().unwrap_or(0))
                        } else {
                            None
                        };
                        let examples = if select.8 {
                            d.0.examples.and_then(|eg| {
                                (rmp_serde::decode::from_slice(&eg)
                                    as Result<Vec<Example>, rmp_serde::decode::Error>)
                                    .ok()
                            })
                        } else {
                            None
                        };
                        let time = if select.9 {
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
                            examples,
                            time,
                        )
                    })
                    .collect();
                Ok(clusters)
            });

        future::result(cluster)
    }

    pub fn execute_select_outlier_query(
        &self,
        data_source: &str,
    ) -> impl Future<Item = Vec<OutliersTable>, Error = Error> {
        use schema::Outliers::dsl::*;
        let result = self.pool.get().map_err(Into::into).and_then(|conn| {
            Outliers
                .filter(outlier_data_source.eq(data_source))
                .load::<OutliersTable>(&conn)
                .map_err(Into::into)
        });

        future::result(result)
    }

    pub fn add_outliers(
        &self,
        new_outliers: &[OutlierUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        let insert_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            let insert_outliers: Vec<OutliersTable> = new_outliers
                .iter()
                .map(|new_outlier| {
                    let o_size = Some(new_outlier.event_ids.len().to_string());
                    let event_ids = rmp_serde::encode::to_vec(&new_outlier.event_ids).ok();
                    OutliersTable {
                        outlier_id: None,
                        outlier_raw_event: new_outlier.outlier.to_vec(),
                        outlier_data_source: new_outlier.data_source.to_string(),
                        outlier_event_ids: event_ids,
                        outlier_size: o_size,
                    }
                })
                .collect();

            if !insert_outliers.is_empty() {
                diesel::insert_into(schema::Outliers::dsl::Outliers)
                    .values(&insert_outliers)
                    .execute(&*conn)
                    .map_err(Into::into)
            } else {
                Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::Other,
                )))
            }
        });
        future::result(insert_result)
    }

    pub fn update_outliers(
        &self,
        outlier_update: &[OutlierUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::Outliers::dsl::*;
        let mut query = Outliers.into_boxed();
        for outlier in outlier_update.iter() {
            query = query.or_filter(
                outlier_raw_event
                    .eq(&outlier.outlier)
                    .and(outlier_data_source.eq(&outlier.data_source)),
            );
        }
        let execute_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            query
                .load::<OutliersTable>(&conn)
                .map_err(Into::into)
                .and_then(|outlier_list| {
                    let mut update_queries = String::new();
                    for o in outlier_update {
                        if let Some(outlier) = outlier_list
                            .iter()
                            .find(|outlier| o.outlier == outlier.outlier_raw_event)
                        {
                            let new_size = o.event_ids.len();
                            let o_size = match &outlier.outlier_size {
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
                            let mut event_ids = match &outlier.outlier_event_ids {
                                Some(event_ids) => {
                                    match rmp_serde::decode::from_slice(&event_ids)
                                        as Result<Vec<u64>, rmp_serde::decode::Error>
                                    {
                                        Ok(event_ids) => event_ids,
                                        Err(_) => Vec::<u64>::new(),
                                    }
                                }
                                None => Vec::<u64>::new(),
                            };
                            event_ids.extend(&o.event_ids);
                            // only store most recent 100 event_ids per outlier
                            let event_ids = if event_ids.len() > 100 {
                                event_ids.sort();
                                let (_, event_ids) = event_ids.split_at(event_ids.len() - 100);
                                event_ids
                            } else {
                                &event_ids
                            };
                            let event_ids = rmp_serde::encode::to_vec(event_ids).ok();
                            match event_ids {
                                Some(event_ids) => {
                                    let query = format!("INSERT OR REPLACE INTO Outliers (outlier_raw_event, outlier_event_ids, outlier_size, outlier_data_source) VALUES (x'{}', x'{}', '{}', '{}');", 
                                        hex::encode(DB::bytes_to_string(&o.outlier)),
                                        hex::encode(event_ids),
                                        o_size,
                                        o.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                                None => {
                                    let query = format!("INSERT OR REPLACE INTO Outliers (outlier_raw_event, outlier_size, outlier_data_source) VALUES (x'{}', '{}', '{}');", 
                                        hex::encode(DB::bytes_to_string(&o.outlier)),
                                        o_size,
                                        o.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                            }
                        } else {
                            let event_ids = rmp_serde::encode::to_vec(&o.event_ids).ok();
                            match event_ids {
                                Some(event_ids) => {
                                    let query = format!("INSERT OR REPLACE INTO Outliers (outlier_raw_event, outlier_event_ids, outlier_size, outlier_data_source) VALUES (x'{}', x'{}', '{}', '{}');", 
                                        hex::encode(DB::bytes_to_string(&o.outlier)),
                                        hex::encode(event_ids),
                                        o.event_ids.len(),
                                        o.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                                None => {
                                    let query = format!("INSERT OR REPLACE INTO Outliers (outlier_raw_event, outlier_size, outlier_data_source) VALUES (x'{}', '{}', '{}');", 
                                        hex::encode(DB::bytes_to_string(&o.outlier)),
                                        o.event_ids.len(),
                                        o.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                            }
                        }
                    }
                    if !update_queries.is_empty() {
                        conn.execute(&update_queries).map_err(Into::into)
                    } else {
                        Err(Error::from(ErrorKind::DatabaseTransactionError(
                            DatabaseError::Other,
                        )))
                    }
                })
        });
        future::result(execute_result)
    }

    pub fn update_qualifiers(
        &self,
        qualifier_update: &[QualifierUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::Clusters::dsl;
        let result = self.pool.get().map_err(Into::into).and_then(|conn| {
            let timestamp =
                chrono::NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0);
            let row = qualifier_update
                .iter()
                .map(|q| {
                    if let Ok(Some(qualifier_id)) = DB::get_qualifier_id(&self, &q.qualifier) {
                        let target = dsl::Clusters.filter(
                            dsl::cluster_id
                                .eq(&q.cluster_id)
                                .and(dsl::data_source.eq(&q.data_source)),
                        );
                        diesel::update(target)
                            .set((
                                dsl::qualifier_id.eq(qualifier_id),
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
                Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::Other,
                )))
            } else {
                Ok(row)
            }
        });

        future::result(result)
    }

    fn get_category_id(&self, category: &str) -> Result<Option<i32>, Error> {
        use schema::Category::dsl;
        self.pool.get().map_err(Into::into).and_then(|conn| {
            dsl::Category
                .select(dsl::category_id)
                .filter(dsl::category.eq(category))
                .first::<Option<i32>>(&conn)
                .map_err(Into::into)
        })
    }

    fn get_qualifier_id(&self, qualifier: &str) -> Result<Option<i32>, Error> {
        use schema::Qualifier::dsl;
        self.pool.get().map_err(Into::into).and_then(|conn| {
            dsl::Qualifier
                .select(dsl::qualifier_id)
                .filter(dsl::qualifier.eq(qualifier))
                .first::<Option<i32>>(&conn)
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
        use schema::Clusters::dsl;

        let query = diesel::update(dsl::Clusters).filter(
            dsl::cluster_id
                .eq(cluster_id)
                .and(dsl::data_source.eq(data_source)),
        );
        let timestamp = chrono::NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0);
        let category_id = category.and_then(|category| DB::get_category_id(self, &category).ok());
        let (qualifier_id, is_benign) = qualifier.map_or((None, false), |qualifier| {
            (
                DB::get_qualifier_id(self, &qualifier).ok(),
                qualifier == "benign",
            )
        });
        let execution_result = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| {
                if let (Some(cluster_id), Some(Some(category_id)), Some(Some(qualifier_id))) =
                    (&new_cluster_id, &category_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::category_id.eq(category_id),
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let (Some(cluster_id), Some(Some(category_id))) =
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
                } else if let (Some(cluster_id), Some(Some(qualifier_id))) =
                    (&new_cluster_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let (Some(Some(category_id)), Some(Some(qualifier_id))) =
                    (&category_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::category_id.eq(category_id),
                            dsl::qualifier_id.eq(qualifier_id),
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
                } else if let Some(Some(category_id)) = &category_id {
                    query
                        .set((
                            dsl::category_id.eq(category_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let Some(Some(qualifier_id)) = &qualifier_id {
                    query
                        .set((
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else {
                    Err(Error::from(ErrorKind::DatabaseTransactionError(
                        DatabaseError::Other,
                    )))
                }
            })
            .and_then(|row| Ok((row as u8, is_benign, data_source.to_string())));

        future::result(execution_result)
    }

    pub fn update_clusters(
        &self,
        cluster_update: &[ClusterUpdate],
    ) -> future::FutureResult<usize, Error> {
        use schema::Clusters::dsl::*;
        use serde::Serialize;
        #[derive(Debug, Queryable, Serialize)]
        pub struct Cluster {
            cluster_id: Option<String>,
            signature: String,
            examples: Option<Vec<u8>>,
            size: String,
            category_id: i32,
            priority_id: i32,
            qualifier_id: i32,
            status_id: i32,
        }
        let mut query = Clusters.into_boxed();
        for cluster in cluster_update.iter() {
            query = query.or_filter(
                cluster_id
                    .eq(&cluster.cluster_id)
                    .and(data_source.eq(&cluster.data_source)),
            );
        }
        let execution_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            query
                .select((
                    cluster_id,
                    signature,
                    examples,
                    size,
                    category_id,
                    priority_id,
                    qualifier_id,
                    status_id,
                ))
                .load::<Cluster>(&conn)
                .map_err(Into::into)
                .and_then(|cluster_list| {
                    let mut update_queries = String::new();
                    for c in cluster_update {
                        if let Some(cluster) = cluster_list
                            .iter()
                            .find(|cluster| Some(c.cluster_id.clone()) == cluster.cluster_id)
                        {
                            let now = chrono::Utc::now();
                            let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
                            if c.signature.is_some() || c.examples.is_some() || c.size.is_some() {
                                let sig = match &c.signature {
                                    Some(sig) => sig.clone(),
                                    None => cluster.signature.clone(),
                                };
                                let example =
                                    DB::merge_cluster_examples(cluster.examples.clone(), c.examples.clone());
                                let cluster_size = match c.size {
                                    Some(new_size) => {
                                        if let Ok(current_size) = cluster.size.clone().parse::<usize>() {
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
                                match example {
                                    Some(example) => {
                                        let query = format!("INSERT OR REPLACE INTO Clusters (cluster_id, category_id, detector_id, examples, priority_id, qualifier_id, status_id, rules, signature, size, data_source, last_modification_time) VALUES ('{}', {}, '{}', x'{}', {}, {}, {}, '{}', '{}', '{}', '{}', '{}');", 
                                            c.cluster_id,
                                            cluster.category_id,
                                            c.detector_id,
                                            hex::encode(example),
                                            cluster.priority_id,
                                            cluster.qualifier_id,
                                            cluster.status_id,
                                            sig,
                                            sig,
                                            cluster_size,
                                            c.data_source,
                                            timestamp,
                                        );
                                      update_queries.push_str(&query);
                                    }
                                    None => {
                                        let query = format!("INSERT OR REPLACE INTO Clusters (cluster_id, category_id, detector_id, priority_id, qualifier_id, status_id, rules, signature, size, data_source, last_modification_time) VALUES ('{}', {}, '{}', {}, {}, {}, '{}', '{}', '{}', '{}', '{}');", 
                                            c.cluster_id,
                                            cluster.category_id,
                                            c.detector_id,
                                            cluster.priority_id,
                                            cluster.qualifier_id,
                                            cluster.status_id,
                                            sig,
                                            sig,
                                            cluster_size,
                                            c.data_source,
                                            timestamp,
                                        );
                                       update_queries.push_str(&query);
                                    }
                                }
                            }
                        } else {
                            let example = match &c.examples {
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
                            match example {
                                Some(example) => {
                                    let query = format!("INSERT OR REPLACE INTO Clusters (cluster_id, category_id, detector_id, examples, priority_id, qualifier_id, status_id, rules, signature, size, data_source) VALUES ('{}', {}, '{}', x'{}', {}, {}, {}, '{}', '{}', '{}', '{}');", 
                                        c.cluster_id,
                                        1,
                                        c.detector_id,
                                        hex::encode(example),
                                        1,
                                        2,
                                        2,
                                        sig,
                                        sig,
                                        cluster_size,
                                        c.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                                None => {
                                    let query = format!("INSERT OR REPLACE INTO Clusters (cluster_id, category_id, detector_id, priority_id, qualifier_id, status_id, rules, signature, size, data_source) VALUES ('{}', {}, '{}', {}, {}, {}, '{}', '{}', '{}', '{}');", 
                                        c.cluster_id,
                                        1,
                                        c.detector_id,
                                        1,
                                        2,
                                        2,
                                        sig,
                                        sig,
                                        cluster_size,
                                        c.data_source,
                                    );
                                    update_queries.push_str(&query);
                                }
                            }
                        }
                    }

                    if !update_queries.is_empty() {
                        conn.execute(&update_queries).map_err(Into::into)
                    } else {
                        Err(Error::from(ErrorKind::DatabaseTransactionError(
                            DatabaseError::Other,
                        )))
                    }
                })
        });

        future::result(execution_result)
    }

    pub fn add_clusters(
        &self,
        new_clusters: &[ClusterUpdate],
    ) -> impl Future<Item = usize, Error = Error> {
        let insert_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            let insert_clusters: Vec<ClustersTable> = new_clusters
                .iter()
                .map(|c| {
                    let example = match &c.examples {
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
                    // We always insert 1 for category_id and priority_id,
                    // "unknown" for qualifier_id, and "pending review" for status_id.
                    ClustersTable {
                        id: None,
                        cluster_id: Some(c.cluster_id.to_string()),
                        description: None,
                        category_id: 1,
                        detector_id: c.detector_id,
                        examples: example,
                        priority_id: 1,
                        qualifier_id: 2,
                        status_id: 2,
                        rules: Some(sig.clone()),
                        signature: sig,
                        size: cluster_size,
                        data_source: c.data_source.to_string(),
                        last_modification_time: None,
                    }
                })
                .collect();

            if !insert_clusters.is_empty() {
                diesel::insert_into(schema::Clusters::dsl::Clusters)
                    .values(&insert_clusters)
                    .execute(&*conn)
                    .map_err(Into::into)
            } else {
                Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::Other,
                )))
            }
        });

        future::result(insert_result)
    }

    fn merge_cluster_examples(
        current_examples: Option<Vec<u8>>,
        new_examples: Option<Vec<Example>>,
    ) -> Option<Vec<u8>> {
        let max_examples_num: usize = 25;

        match new_examples {
            Some(new_eg) => {
                if new_eg.len() >= max_examples_num {
                    match rmp_serde::encode::to_vec(&new_eg) {
                        Ok(new_eg) => Some(new_eg),
                        Err(_) => current_examples,
                    }
                } else if let Some(current_eg) = current_examples.clone() {
                    match rmp_serde::decode::from_slice(&current_eg)
                        as Result<Vec<Example>, rmp_serde::decode::Error>
                    {
                        Ok(mut current_eg) => {
                            current_eg.extend(new_eg);
                            if current_eg.len() > max_examples_num {
                                current_eg.sort();
                                let (_, current_eg) =
                                    current_eg.split_at(current_eg.len() - max_examples_num);
                                match rmp_serde::encode::to_vec(&current_eg) {
                                    Ok(eg) => Some(eg),
                                    Err(_) => current_examples,
                                }
                            } else {
                                match rmp_serde::encode::to_vec(&current_eg) {
                                    Ok(eg) => Some(eg),
                                    Err(_) => current_examples,
                                }
                            }
                        }
                        Err(_) => current_examples,
                    }
                } else {
                    None
                }
            }
            None => current_examples,
        }
    }

    pub fn update_category(
        &self,
        current_category: &str,
        new_category: &str,
    ) -> impl Future<Item = usize, Error = Error> {
        use schema::Category::dsl::*;
        let update_result = DB::get_category_id(&self, current_category).and_then(|_| {
            let target = Category.filter(category.eq(current_category));
            self.pool.get().map_err(Into::into).and_then(|conn| {
                diesel::update(target)
                    .set((category.eq(new_category),))
                    .execute(&conn)
                    .map_err(Into::into)
            })
        });
        future::result(update_result)
    }

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
    }
}
