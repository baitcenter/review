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

    pub fn update_cluster(
        &self,
        c_id: &str,
        d_id: i32,
        rule: &Option<String>,
        _size: Option<usize>,
        sig: &Option<String>,
        eg: &Option<Vec<String>>,
    ) -> Box<Future<Item = (), Error = Error>> {
        let conn = self.pool.get().unwrap();
        let record_check = Events
            .filter(schema::Events::dsl::detector_id.eq(d_id))
            .filter(schema::Events::dsl::cluster_id.eq(c_id))
            .load::<EventsTable>(&conn);

        if record_check.is_ok() {
            if record_check.unwrap().is_empty() {
                let unknown = Qualifier.load::<QualifierTable>(&conn);
                let review = Status.load::<StatusTable>(&conn);
                let (review, unknown) = if let (Ok(review), Ok(unknown)) = (review, unknown) {
                    (review, unknown)
                } else {
                    return Box::new(futures::future::ok(()));
                };
                let review = review.iter().find(|x| x.status == "review");
                let unknown = unknown.iter().find(|x| x.qualifier == "unknown");

                let example = if let Some(eg) = eg {
                    Some(eg.join("\n"))
                } else {
                    None
                };

                // We always insert 1 for category_id and priority_id,
                // "unknown" for qualifier_id, and "review" for status_id.
                // We also assume that REconverge alway sends signatures
                // for new clusters
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
                    rules: rule.clone(),
                    signature: sig.clone().unwrap(),
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
                if rule.is_some() {
                    let _ = diesel::update(target)
                        .set((
                            schema::Events::dsl::rules.eq(rule.clone()),
                            schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                        ))
                        .execute(&conn);
                }
                if let Some(sig) = sig {
                    let _ = diesel::update(target)
                        .set((
                            schema::Events::dsl::signature.eq(sig.clone()),
                            schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                        ))
                        .execute(&conn);
                }
                if let Some(example) = eg {
                    let example = Some(example.join("\n"));
                    let _ = diesel::update(target)
                        .set((
                            schema::Events::dsl::examples.eq(example),
                            schema::Events::dsl::last_modification_time.eq(Some(timestamp)),
                        ))
                        .execute(&conn);
                }
            }
        }
        Box::new(futures::future::ok(()))
    }

    pub fn update_qualifier_id(
        &self,
        id: i32,
        new_qualifier_id: i32,
    ) -> impl Future<Item = usize, Error = Error> {
        let conn = self.pool.get().unwrap();
        DB::get_status_table(&self)
            .and_then(move |status_table| {
                let active = status_table.iter().find(|x| x.status == "active");
                let result = Events
                    .filter(schema::Events::dsl::event_id.eq(id))
                    .load::<EventsTable>(&conn)
                    .map_err(Into::into)
                    .and_then(|mut event| {
                        event[0].event_id = None;
                        event[0].qualifier_id = new_qualifier_id;
                        event[0].status_id = active.unwrap().status_id.unwrap();
                        let now = chrono::Utc::now();
                        event[0].last_modification_time =
                            Some(chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0));
                        diesel::update(Events.find(id))
                            .set(&event[0])
                            .execute(&conn)
                            .map_err(Into::into)
                    });

                future::result(result)
            })
            .map_err(Into::into)
    }
}
