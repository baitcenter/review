#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
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
pub use error::Error;

mod models;
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

    pub fn get_event_table(&self) -> impl Future<Item = Vec<EventsTable>, Error = Error> {
        let event_table = self
            .pool
            .get()
            .map_err(Into::into)
            .and_then(|conn| Events.load::<EventsTable>(&conn).map_err(Into::into));

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

    pub fn get_event_by_status(
        &self,
        s_id: i32,
    ) -> impl Future<Item = Vec<EventsTable>, Error = Error> {
        let event = self.pool.get().map_err(Into::into).and_then(|conn| {
            Events
                .filter(schema::Events::dsl::status_id.eq(s_id))
                .load::<EventsTable>(&conn)
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

    pub fn update_outlier(
        &self,
        outlier: &[u8],
        datasource: &str,
        new_event_ids: &[u64],
    ) -> Box<Future<Item = (), Error = Error>> {
        let conn = self.pool.get().unwrap();
        let record_check = Outliers
            .filter(schema::Outliers::dsl::outlier_raw_event.eq(outlier))
            .filter(schema::Outliers::dsl::outlier_data_source.eq(datasource))
            .load::<OutliersTable>(&conn);

        if let Ok(record_check) = record_check {
            if record_check.is_empty() {
                let o_size = Some(new_event_ids.len().to_string());
                let event_ids = rmp_serde::encode::to_vec(new_event_ids).ok();
                let o = OutliersTable {
                    outlier_id: None,
                    outlier_raw_event: outlier.to_vec(),
                    outlier_data_source: datasource.to_string(),
                    outlier_event_ids: event_ids,
                    outlier_size: o_size,
                };
                let _ = diesel::insert_into(Outliers).values(&o).execute(&conn);
            } else {
                let new_size = new_event_ids.len();
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
                event_ids.extend(new_event_ids);
                // only store most recent 100 event_ids per outlier
                let event_ids = if event_ids.len() > 100 {
                    event_ids.sort();
                    let (_, event_ids) = event_ids.split_at(event_ids.len() - 100);
                    event_ids
                } else {
                    &event_ids
                };
                let event_ids = rmp_serde::encode::to_vec(event_ids).ok();
                let target = Outliers
                    .filter(schema::Outliers::dsl::outlier_raw_event.eq(outlier))
                    .filter(schema::Outliers::dsl::outlier_data_source.eq(datasource));
                let _ = diesel::update(target)
                    .set((
                        schema::Outliers::dsl::outlier_event_ids.eq(event_ids),
                        schema::Outliers::dsl::outlier_size.eq(o_size),
                    ))
                    .execute(&conn);
            }
        }
        Box::new(futures::future::ok(()))
    }

    pub fn update_cluster(
        &self,
        c_id: &str,
        d_id: i32,
        sig: &Option<String>,
        datasource: &str,
        cluster_size: Option<usize>,
        eg: &Option<Vec<(usize, String)>>,
    ) -> Box<Future<Item = (), Error = Error>> {
        let conn = self.pool.get().unwrap();
        let record_check = Events
            .filter(schema::Events::dsl::detector_id.eq(d_id))
            .filter(schema::Events::dsl::cluster_id.eq(c_id))
            .load::<EventsTable>(&conn);

        if let Ok(record_check) = record_check {
            if record_check.is_empty() {
                let unknown = Qualifier.load::<QualifierTable>(&conn);
                let review = Status.load::<StatusTable>(&conn);
                let (review, unknown) = if let (Ok(review), Ok(unknown)) = (review, unknown) {
                    (review, unknown)
                } else {
                    return Box::new(futures::future::ok(()));
                };
                let review = review.iter().find(|x| x.status == "review");
                let unknown = unknown.iter().find(|x| x.qualifier == "unknown");
                let example = match eg {
                    Some(eg) => rmp_serde::encode::to_vec(eg).ok(),
                    None => None,
                };

                // Signature is required field in central repo database
                // but if new cluster information does not have signature field,
                // we use '-' as a signature
                let sig = match sig {
                    Some(sig) => sig.clone(),
                    None => "-".to_string(),
                };
                let cluster_size = match cluster_size {
                    Some(cluster_size) => cluster_size.to_string(),
                    None => "1".to_string(),
                };
                // We always insert 1 for category_id and priority_id,
                // "unknown" for qualifier_id, and "review" for status_id.
                let event = EventsTable {
                    event_id: None,
                    cluster_id: Some(c_id.to_string()),
                    description: None,
                    category_id: 1,
                    detector_id: d_id,
                    examples: example,
                    priority_id: 1,
                    qualifier_id: unknown.unwrap().qualifier_id.unwrap(),
                    status_id: review.unwrap().status_id.unwrap(),
                    rules: Some(sig.clone()),
                    signature: sig,
                    size: cluster_size,
                    data_source: datasource.to_string(),
                    last_modification_time: None,
                };
                let _ = diesel::insert_into(Events).values(&event).execute(&conn);
                return Box::new(futures::future::ok(()));
            } else {
                let target = Events
                    .filter(schema::Events::dsl::detector_id.eq(d_id))
                    .filter(schema::Events::dsl::cluster_id.eq(c_id));
                let now = chrono::Utc::now();
                let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
                if sig.is_some() || eg.is_some() || cluster_size.is_some() {
                    let sig = match sig {
                        Some(sig) => sig.clone(),
                        None => record_check[0].signature.clone(),
                    };
                    let example =
                        DB::merge_cluster_examples(record_check[0].examples.clone(), eg.clone());
                    let cluster_size = match cluster_size {
                        Some(new_size) => {
                            if let Ok(current_size) = record_check[0].size.clone().parse::<usize>()
                            {
                                // check if sum of new_size and current_size exceeds max_value
                                // if it does, we cannot calculate sum anymore, so reset the value of size
                                if new_size > usize::max_value() - current_size {
                                    new_size.to_string()
                                } else {
                                    (current_size + new_size).to_string()
                                }
                            } else {
                                record_check[0].size.clone()
                            }
                        }
                        None => record_check[0].size.clone(),
                    };
                    let _ = diesel::update(target)
                        .set((
                            schema::Events::dsl::rules.eq(sig.clone()),
                            schema::Events::dsl::signature.eq(sig),
                            schema::Events::dsl::examples.eq(example),
                            schema::Events::dsl::size.eq(cluster_size),
                            schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                        ))
                        .execute(&conn);
                }
            }
        }
        Box::new(futures::future::ok(()))
    }

    fn merge_cluster_examples(
        current_examples: Option<Vec<u8>>,
        new_examples: Option<Vec<(usize, String)>>,
    ) -> Option<Vec<u8>> {
        const MAX_EXAMPLES_NUM: usize = 25;

        match new_examples {
            Some(new_eg) => {
                if new_eg.len() >= MAX_EXAMPLES_NUM {
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
                            if current_eg.len() > MAX_EXAMPLES_NUM {
                                current_eg.sort();
                                let (_, current_eg) =
                                    current_eg.split_at(current_eg.len() - MAX_EXAMPLES_NUM);

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

    pub fn update_qualifier_id(
        &self,
        e_id: i32,
        new_qualifier_id: i32,
    ) -> Box<Future<Item = i8, Error = Error> + Send + 'static> {
        let conn = self.pool.get().unwrap();

        let q_id_check = Qualifier
            .filter(schema::Qualifier::dsl::qualifier_id.eq(new_qualifier_id))
            .load::<QualifierTable>(&conn);
        if q_id_check.is_ok() && q_id_check.unwrap().is_empty() {
            return Box::new(futures::future::ok(-1));
        }

        let active = match Status.load::<StatusTable>(&conn) {
            Ok(status_table) => {
                let active = status_table.iter().find(|x| x.status == "active");
                active.unwrap().status_id.unwrap()
            }
            Err(_) => return Box::new(futures::future::ok(-1)),
        };
        let record_check = Events
            .filter(schema::Events::dsl::event_id.eq(e_id))
            .load::<EventsTable>(&conn);

        if record_check.is_ok() && !record_check.unwrap().is_empty() {
            let target = Events.filter(schema::Events::dsl::event_id.eq(e_id));
            let now = chrono::Utc::now();
            let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
            let _ = diesel::update(target)
                .set((
                    schema::Events::dsl::qualifier_id.eq(new_qualifier_id),
                    schema::Events::dsl::status_id.eq(active),
                    schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                ))
                .execute(&conn);

            return Box::new(futures::future::ok(0));
        }

        Box::new(futures::future::ok(-1))
    }
}
