#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use failure::Fail;
use futures::future;
use futures::prelude::*;
use models::*;
use schema::Action::dsl::*;
use schema::Category::dsl::*;
use schema::Events::dsl::*;
use schema::Outliers::dsl::*;
use schema::Priority::dsl::*;
use schema::Qualifier::dsl::*;
use schema::Status::dsl::*;

pub mod error;
pub use self::error::{DatabaseError, Error, ErrorKind};
pub mod models;
mod schema;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

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

    pub fn add_new_category(&self, new_category: &str) -> impl Future<Item = (), Error = Error> {
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

    pub fn get_action_table(&self) -> impl Future<Item = Vec<ActionTable>, Error = Error> {
        let action_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Action.load::<ActionTable>(&conn).map_err(Into::into));

        future::result(action_table)
    }

    pub fn get_category_table(&self) -> impl Future<Item = Vec<CategoryTable>, Error = Error> {
        let category_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Category.load::<CategoryTable>(&conn).map_err(Into::into));

        future::result(category_table)
    }

    pub fn get_event_table(
        &self,
    ) -> impl Future<Item = Vec<(EventsTable, StatusTable, QualifierTable, CategoryTable)>, Error = Error>
    {
        let event_table = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .inner_join(Status)
                .inner_join(Qualifier)
                .inner_join(Category)
                .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                .map_err(Into::into)
        });

        future::result(event_table)
    }

    pub fn get_priority_table(&self) -> impl Future<Item = Vec<PriorityTable>, Error = Error> {
        let priority_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Priority.load::<PriorityTable>(&conn).map_err(Into::into));

        future::result(priority_table)
    }

    pub fn get_qualifier_table(&self) -> impl Future<Item = Vec<QualifierTable>, Error = Error> {
        let qualifier_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Qualifier.load::<QualifierTable>(&conn).map_err(Into::into));

        future::result(qualifier_table)
    }

    pub fn get_status_table(&self) -> impl Future<Item = Vec<StatusTable>, Error = Error> {
        let status_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Status.load::<StatusTable>(&conn).map_err(Into::into));

        future::result(status_table)
    }

    pub fn get_event_by_category(
        &self,
        cat_id: &str,
    ) -> impl Future<Item = Vec<(EventsTable, StatusTable, QualifierTable, CategoryTable)>, Error = Error>
    {
        if cat_id.contains(',') {
            let cat_id = cat_id.split(',');
            let cat_id_vec = cat_id
                .map(|c| (&c).parse::<i32>())
                .filter_map(Result::ok)
                .collect::<Vec<_>>();
            if !cat_id_vec.is_empty() && cat_id_vec.len() != 1 {
                let mut query = String::new();
                for (index, cat_id) in cat_id_vec.iter().enumerate() {
                    if index == 0 {
                        let q = format!("Events.category_id = {}", cat_id);
                        query.push_str(&q);
                    } else {
                        let q = format!(" or Events.category_id = {}", cat_id);
                        query.push_str(&q);
                    }
                }
                let query = format!(
                    "SELECT * FROM Events INNER JOIN Category ON Events.category_id = Category.category_id INNER JOIN Qualifier ON Events.qualifier_id = Qualifier.qualifier_id INNER JOIN Status ON Events.status_id = Status.status_id WHERE {};",
                    query
                );
                let event = self.pool.get().map_err(Into::into).and_then(|conn| {
                    diesel::sql_query(query)
                        .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                        .map_err(Into::into)
                });
                future::result(event)
            } else if cat_id_vec.len() == 1 {
                let event = self.pool.get().map_err(Into::into).and_then(|conn| {
                    Events
                        .filter(schema::Events::dsl::category_id.eq(cat_id_vec[0]))
                        .inner_join(Status)
                        .inner_join(Qualifier)
                        .inner_join(Category)
                        .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                        .map_err(Into::into)
                });
                future::result(event)
            } else {
                future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::Other,
                ))))
            }
        } else if let Ok(cat_id) = cat_id.parse::<i32>() {
            let event = self.pool.get().map_err(Into::into).and_then(|conn| {
                Events
                    .filter(schema::Events::dsl::category_id.eq(cat_id))
                    .inner_join(Status)
                    .inner_join(Qualifier)
                    .inner_join(Category)
                    .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                    .map_err(Into::into)
            });
            future::result(event)
        } else {
            future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::Other,
            ))))
        }
    }

    pub fn get_event_by_status(
        &self,
        s_id: i32,
    ) -> impl Future<Item = Vec<(EventsTable, StatusTable, QualifierTable, CategoryTable)>, Error = Error>
    {
        let event = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .filter(schema::Events::dsl::status_id.eq(s_id))
                .inner_join(Status)
                .inner_join(Qualifier)
                .inner_join(Category)
                .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                .map_err(Into::into)
        });

        future::result(event)
    }

    pub fn get_event_by_data_source(
        &self,
        datasource: &str,
    ) -> impl Future<Item = Vec<(EventsTable, StatusTable, QualifierTable, CategoryTable)>, Error = Error>
    {
        let event = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .filter(schema::Events::dsl::data_source.eq(datasource))
                .inner_join(Status)
                .inner_join(Qualifier)
                .inner_join(Category)
                .load::<(EventsTable, StatusTable, QualifierTable, CategoryTable)>(&conn)
                .map_err(Into::into)
        });

        future::result(event)
    }

    pub fn get_benign_id(&self) -> i32 {
        if let Ok(conn) = self.pool.get() {
            if let Ok(qualifier_table) = Qualifier.load::<QualifierTable>(&conn) {
                if let Some(benign) = qualifier_table.iter().find(|x| x.qualifier == "benign") {
                    return benign.qualifier_id.unwrap();
                }
            }
        }
        -1
    }

    pub fn get_cluster(
        &self,
        c_id: &str,
    ) -> impl Future<Item = Vec<ClusterExample>, Error = Error> {
        let cluster = self.pool.get().map_err(Into::into).and_then(|conn| {
            let query = format!(
                "SELECT cluster_id, examples FROM Events WHERE cluster_id = '{}'",
                c_id
            );
            diesel::sql_query(query)
                .load::<ClusterExample>(&conn)
                .map_err(Into::into)
        });

        future::result(cluster)
    }
    pub fn get_cluster_only(
        &self,
        datasource: &str,
    ) -> impl Future<Item = Vec<Option<String>>, Error = Error> {
        let cluster = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .select(schema::Events::dsl::cluster_id)
                .filter(schema::Events::dsl::data_source.eq(datasource))
                .load::<Option<String>>(&conn)
                .map_err(Into::into)
        });

        future::result(cluster)
    }
    pub fn get_cluster_with_limit_num(
        &self,
        c_id: &str,
        limit_num: usize,
    ) -> impl Future<Item = Vec<ClusterExample>, Error = Error> {
        let cluster = self.pool.get().map_err(Into::into).and_then(|conn| {
            let query = format!(
                "SELECT cluster_id, examples FROM Events WHERE cluster_id = '{}' LIMIT {}",
                c_id, limit_num
            );
            diesel::sql_query(query)
                .load::<ClusterExample>(&conn)
                .map_err(Into::into)
        });

        future::result(cluster)
    }

    pub fn get_all_clusters_with_limit_num(
        &self,
        limit_num: usize,
    ) -> impl Future<Item = Vec<ClusterExample>, Error = Error> {
        let cluster = self.pool.get().map_err(Into::into).and_then(|conn| {
            let query = format!(
                "SELECT cluster_id, examples FROM Events LIMIT {}",
                limit_num
            );
            diesel::sql_query(query)
                .load::<ClusterExample>(&conn)
                .map_err(Into::into)
        });

        future::result(cluster)
    }

    pub fn get_signature_by_qualifier(
        &self,
        q_id: i32,
    ) -> impl Future<Item = Vec<String>, Error = Error> {
        let event = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .select(schema::Events::dsl::signature)
                .filter(schema::Events::dsl::qualifier_id.eq(q_id))
                .load::<String>(&conn)
                .map_err(Into::into)
        });

        future::result(event)
    }

    pub fn get_outliers_table(&self) -> impl Future<Item = Vec<OutliersTable>, Error = Error> {
        let outliers_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Outliers.load::<OutliersTable>(&conn).map_err(Into::into));

        future::result(outliers_table)
    }

    pub fn get_outlier_only(
        &self,
        datasource: &str,
    ) -> impl Future<Item = Vec<Vec<u8>>, Error = Error> {
        let cluster = self.pool.get().map_err(Into::into).and_then(|conn| {
            Outliers
                .select(schema::Outliers::dsl::outlier_raw_event)
                .filter(schema::Outliers::dsl::outlier_data_source.eq(datasource))
                .load::<Vec<u8>>(&conn)
                .map_err(Into::into)
        });

        future::result(cluster)
    }

    pub fn add_outliers(
        &self,
        new_outliers: &[OutlierUpdate],
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
        let mut insert_outliers: Vec<OutliersTable> = Vec::new();
        for new_outlier in new_outliers {
            let o_size = Some(new_outlier.event_ids.len().to_string());
            let event_ids = rmp_serde::encode::to_vec(&new_outlier.event_ids).ok();
            insert_outliers.push(OutliersTable {
                outlier_id: None,
                outlier_raw_event: new_outlier.outlier.to_vec(),
                outlier_data_source: new_outlier.data_source.to_string(),
                outlier_event_ids: event_ids,
                outlier_size: o_size,
            });
        }
        let execute_result = if !insert_outliers.is_empty() {
            match diesel::insert_into(Outliers)
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
        update_outliers: &[OutlierUpdate],
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
        let mut update_queries = String::new();

        for o in update_outliers {
            let record_check = Outliers
                .filter(schema::Outliers::dsl::outlier_raw_event.eq(&o.outlier))
                .filter(schema::Outliers::dsl::outlier_data_source.eq(&o.data_source))
                .load::<OutliersTable>(&conn);
            if let Some(record_check) = DB::check_db_query_result(record_check) {
                let new_size = o.event_ids.len();
                let o_size = match &record_check[0].outlier_size {
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
                let mut event_ids = match &record_check[0].outlier_event_ids {
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

    fn check_db_query_result<T>(result: Result<Vec<T>, diesel::result::Error>) -> Option<Vec<T>> {
        match result {
            Ok(result) => {
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
            Err(_) => None,
        }
    }

    pub fn update_clusters(
        &self,
        cluster_update: &[ClusterUpdate],
    ) -> future::FutureResult<(), Error> {
        use schema::*;
        use serde::Serialize;
        #[derive(Debug, Queryable, QueryableByName, Serialize)]
        #[table_name = "Events"]
        pub struct Clusters {
            cluster_id: Option<String>,
            signature: String,
            examples: Option<Vec<u8>>,
            size: String,
            category_id: i32,
            priority_id: i32,
            qualifier_id: i32,
            status_id: i32,
        }
        let conn = self.pool.get().unwrap();
        let mut update_queries = String::new();
        let mut query = String::new();
        for (index, cluster) in cluster_update.iter().enumerate() {
            if index == 0 {
                let q = format!("cluster_id = '{}'", cluster.cluster_id);
                query.push_str(&q);
            } else if index == cluster_update.len() - 1 {
                let q = format!(
                    " or cluster_id = '{}' and data_source = '{}' and detector_id = '{}';",
                    cluster.cluster_id, cluster.data_source, cluster.detector_id,
                );
                query.push_str(&q);
            } else {
                let q = format!(" or cluster_id = '{}'", cluster.cluster_id);
                query.push_str(&q);
            }
        }
        let query = format!(
            "SELECT cluster_id, signature, examples, size, category_id, priority_id, qualifier_id, status_id FROM Events WHERE {}",
            query
        );
        let cluster_list = match diesel::sql_query(query).load::<Clusters>(&conn) {
            Ok(result) => result,
            Err(e) => return future::result(DB::error_handling(e)),
        };

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
                            let query = format!("INSERT OR REPLACE INTO Events (cluster_id, category_id, detector_id, examples, priority_id, qualifier_id, status_id, rules, signature, size, data_source, last_modification_time) VALUES ('{}', {}, '{}', x'{}', {}, {}, {}, '{}', '{}', '{}', '{}', '{}');", 
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
                            let query = format!("INSERT OR REPLACE INTO Events (cluster_id, category_id, detector_id, priority_id, qualifier_id, status_id, rules, signature, size, data_source, last_modification_time) VALUES ('{}', {}, '{}', {}, {}, {}, '{}', '{}', '{}', '{}', '{}');", 
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
                        let query = format!("INSERT OR REPLACE INTO Events (cluster_id, category_id, detector_id, examples, priority_id, qualifier_id, status_id, rules, signature, size, data_source) VALUES ('{}', {}, '{}', x'{}', {}, {}, {}, '{}', '{}', '{}', '{}');", 
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
                        let query = format!("INSERT OR REPLACE INTO Events (cluster_id, category_id, detector_id, priority_id, qualifier_id, status_id, rules, signature, size, data_source) VALUES ('{}', {}, '{}', {}, {}, {}, '{}', '{}', '{}', '{}');", 
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
        let mut insert_clusters: Vec<EventsTable> = Vec::new();
        for c in new_clusters {
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
            let event = EventsTable {
                event_id: None,
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
            };
            insert_clusters.push(event);
        }

        let insert_result = if !insert_clusters.is_empty() {
            match diesel::insert_into(Events)
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
        new_examples: Option<Vec<(usize, String)>>,
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
                        as Result<Vec<(usize, String)>, rmp_serde::decode::Error>
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
        let conn = self.pool.get().unwrap();

        let record_check = Category
            .filter(schema::Category::dsl::category.eq(current_category))
            .load::<CategoryTable>(&conn);
        if DB::check_db_query_result(record_check).is_none() {
            return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            ))));
        }

        let target = Category.filter(schema::Category::dsl::category.eq(current_category));
        let update_result = match diesel::update(target)
            .set((schema::Category::dsl::category.eq(new_category),))
            .execute(&conn)
        {
            Ok(_) => Ok(()),
            Err(e) => DB::error_handling(e),
        };

        future::result(update_result)
    }

    pub fn update_category_in_events(
        &self,
        cls_id: &str,
        datasource: &str,
        cat: &str,
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
        let record_check = Events
            .filter(schema::Events::dsl::cluster_id.eq(cls_id))
            .filter(schema::Events::dsl::data_source.eq(datasource))
            .load::<EventsTable>(&conn);
        if DB::check_db_query_result(record_check).is_none() {
            return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            ))));
        }
        let record_check = Category
            .filter(schema::Category::dsl::category.eq(cat))
            .load::<CategoryTable>(&conn);
        let cat_id = match DB::check_db_query_result(record_check) {
            Some(record_check) => record_check[0].category_id.unwrap(),
            None => {
                return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                    DatabaseError::RecordNotExist,
                ))))
            }
        };
        let target = Events
            .filter(schema::Events::dsl::cluster_id.eq(cls_id))
            .filter(schema::Events::dsl::data_source.eq(datasource));
        let now = chrono::Utc::now();
        let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
        let update_result = match diesel::update(target)
            .set((
                schema::Events::dsl::category_id.eq(cat_id),
                schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
            ))
            .execute(&conn)
        {
            Ok(_) => Ok(()),
            Err(e) => DB::error_handling(e),
        };

        future::result(update_result)
    }

    pub fn update_category_id_in_events(
        &self,
        cls_id: &str,
        datasource: &str,
        cat_id: i32,
    ) -> impl Future<Item = (), Error = Error> {
        let conn = self.pool.get().unwrap();
        let record_check = Events
            .filter(schema::Events::dsl::cluster_id.eq(cls_id))
            .filter(schema::Events::dsl::data_source.eq(datasource))
            .load::<EventsTable>(&conn);
        if DB::check_db_query_result(record_check).is_none() {
            return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            ))));
        }
        let record_check = Category
            .filter(schema::Category::dsl::category_id.eq(cat_id))
            .load::<CategoryTable>(&conn);
        if DB::check_db_query_result(record_check).is_none() {
            return future::result(Err(Error::from(ErrorKind::DatabaseTransactionError(
                DatabaseError::RecordNotExist,
            ))));
        }
        let target = Events
            .filter(schema::Events::dsl::cluster_id.eq(cls_id))
            .filter(schema::Events::dsl::data_source.eq(datasource));
        let now = chrono::Utc::now();
        let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
        let update_result = match diesel::update(target)
            .set((
                schema::Events::dsl::category_id.eq(cat_id),
                schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
            ))
            .execute(&conn)
        {
            Ok(_) => Ok(()),
            Err(e) => DB::error_handling(e),
        };

        future::result(update_result)
    }

    pub fn update_cluster_id(
        &self,
        c_id: &str,
        new_c_id: &str,
    ) -> impl Future<Item = String, Error = Error> {
        let conn = self.pool.get().unwrap();
        let c_id_check = Events
            .filter(schema::Events::dsl::cluster_id.eq(c_id))
            .load::<EventsTable>(&conn);
        let datasource = match DB::check_db_query_result(c_id_check) {
            Some(event) => event[0].data_source.clone(),
            None => return future::result(Ok("No entry found".to_string())),
        };

        let target = Events.filter(schema::Events::dsl::cluster_id.eq(c_id));
        let now = chrono::Utc::now();
        let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
        let update_result = match diesel::update(target)
            .set((
                schema::Events::dsl::cluster_id.eq(new_c_id),
                schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
            ))
            .execute(&conn)
        {
            Ok(_) => Ok(datasource),
            Err(e) => DB::error_handling(e),
        };

        future::result(update_result)
    }

    pub fn update_qualifier_id(
        &self,
        c_id: &str,
        new_qualifier_id: i32,
    ) -> impl Future<Item = i8, Error = Error> {
        let conn = self.pool.get().unwrap();

        let q_id_check = Qualifier
            .filter(schema::Qualifier::dsl::qualifier_id.eq(new_qualifier_id))
            .load::<QualifierTable>(&conn);
        if DB::check_db_query_result(q_id_check).is_none() {
            return future::result(Ok(-1));
        }
        let reviewed = Status
            .filter(schema::Status::dsl::status.eq("reviewed"))
            .load::<StatusTable>(&conn);
        let reviewed = match DB::check_db_query_result(reviewed) {
            Some(reviewed) => reviewed[0].status_id.unwrap(),
            None => return future::result(Ok(-1)),
        };

        let record_check = Events
            .filter(schema::Events::dsl::cluster_id.eq(c_id))
            .load::<EventsTable>(&conn);
        if record_check.is_ok() && !record_check.unwrap().is_empty() {
            let target = Events.filter(schema::Events::dsl::cluster_id.eq(c_id));
            let now = chrono::Utc::now();
            let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
            let update_result = match diesel::update(target)
                .set((
                    schema::Events::dsl::qualifier_id.eq(new_qualifier_id),
                    schema::Events::dsl::status_id.eq(reviewed),
                    schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                ))
                .execute(&conn)
            {
                Ok(_) => Ok(0),
                Err(e) => DB::error_handling(e),
            };

            return future::result(update_result);
        }

        future::result(Ok(-1))
    }

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
    }
}
