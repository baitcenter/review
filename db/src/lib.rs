#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use failure::Fail;
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

    pub fn add_category(&self, new_category: &str) -> impl Future<Item = (), Error = Error> {
        use schema::Category::dsl::*;
        let conn = self.pool.get().unwrap();
        let c = CategoryTable {
            category_id: None,
            category: new_category.to_string(),
        };
        let insert_result = match diesel::insert_into(Category).values(&c).execute(&conn) {
            Ok(_) => Ok(()),
            Err(e) => return future::result(DB::error_handling(e)),
        };

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

    pub fn execute_update_query(&self, query: &str) -> future::FutureResult<(), Error> {
        let conn = self.pool.get().unwrap();
        let execution_result = match conn.execute(query) {
            Ok(_) => Ok(()),
            Err(e) => DB::error_handling(e),
        };
        future::result(execution_result)
    }

    pub fn add_outliers(
        &self,
        new_outliers: &[OutlierUpdate],
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
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

        let execute_result = if !insert_outliers.is_empty() {
            match diesel::insert_into(schema::Outliers::dsl::Outliers)
                .values(&insert_outliers)
                .execute(&*conn)
            {
                Ok(_) => Ok(()),
                Err(e) => DB::error_handling(e),
            }
        } else {
            Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::Other,
            )))
        };
        future::result(execute_result)
    }

    pub fn update_outliers(
        &self,
        outlier_update: &[OutlierUpdate],
    ) -> impl Future<Item = (), Error = Error> {
        use schema::Outliers::dsl::*;
        let mut query = Outliers.into_boxed();
        for outlier in outlier_update.iter() {
            query = query.or_filter(
                outlier_raw_event
                    .eq(&outlier.outlier)
                    .and(outlier_data_source.eq(&outlier.data_source)),
            );
        }
        let conn = self.pool.get().unwrap();
        let outlier_list = match query.load::<OutliersTable>(&conn) {
            Ok(result) => result,
            Err(e) => return future::result(DB::error_handling(e)),
        };
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
        let execute_result = if !update_queries.is_empty() {
            match conn.execute(&update_queries) {
                Ok(_) => Ok(()),
                Err(e) => DB::error_handling(e),
            }
        } else {
            Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            )))
        };
        future::result(execute_result)
    }

    pub fn update_clusters(
        &self,
        cluster_update: &[ClusterUpdate],
    ) -> future::FutureResult<(), Error> {
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
        let conn = self.pool.get().unwrap();
        let cluster_list = match query
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
        {
            Ok(result) => result,
            Err(e) => return future::result(DB::error_handling(e)),
        };
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

        let execution_result = if !update_queries.is_empty() {
            match conn.execute(&update_queries) {
                Ok(_) => Ok(()),
                Err(e) => DB::error_handling(e),
            }
        } else {
            Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            )))
        };
        future::result(execution_result)
    }

    pub fn add_clusters(
        &self,
        new_clusters: &[ClusterUpdate],
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
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
        let insert_result = if !insert_clusters.is_empty() {
            match diesel::insert_into(schema::Clusters::dsl::Clusters)
                .values(&insert_clusters)
                .execute(&*conn)
            {
                Ok(_) => Ok(()),
                Err(e) => DB::error_handling(e),
            }
        } else {
            Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::Other,
            )))
        };
        future::result(insert_result)
    }

    fn error_handling<T>(e: diesel::result::Error) -> Result<T, error::Error> {
        match e {
            diesel::result::Error::DatabaseError(_, _) => {
                if e.to_string().contains("database is locked") {
                    Err(Error::from(e.context(ErrorKind::DatabaseTransactionError(
                        DatabaseError::DatabaseLocked,
                    ))))
                } else {
                    Err(Error::from(e.context(ErrorKind::DatabaseTransactionError(
                        DatabaseError::Other,
                    ))))
                }
            }
            _ => Err(Error::from(e.context(ErrorKind::DatabaseTransactionError(
                DatabaseError::Other,
            )))),
        }
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
    ) -> impl Future<Item = (), Error = Error> {
        use schema::Category::dsl::*;

        let record_check: Result<String, Error> =
            self.pool.get().map_err(Into::into).and_then(|conn| {
                Category
                    .select(category)
                    .filter(category.eq(current_category))
                    .first::<String>(&conn)
                    .map_err(Into::into)
            });
        if let Err(e) = record_check {
            if e.to_string().contains("database query") {
                return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::RecordNotExist,
                ))));
            } else {
                return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::Other,
                ))));
            }
        }

        let target = Category.filter(category.eq(current_category));
        let update_result = self.pool.get().map_err(Into::into).and_then(|conn| {
            match diesel::update(target)
                .set((category.eq(new_category),))
                .execute(&conn)
            {
                Ok(_) => Ok(()),
                Err(e) => DB::error_handling(e),
            }
        });
        future::result(update_result)
    }

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
    }
}
