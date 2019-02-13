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
        let conn = self.pool.get().unwrap();
        let action_table = Action.load::<ActionTable>(&conn).map_err(Into::into);

        future::result(action_table)
    }

    pub fn get_category_table(&self) -> impl Future<Item = Vec<CategoryTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let category_table = Category.load::<CategoryTable>(&conn).map_err(Into::into);

        future::result(category_table)
    }

    pub fn get_event_table(&self) -> impl Future<Item = Vec<EventsTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let event_table = Events.load::<EventsTable>(&conn).map_err(Into::into);

        future::result(event_table)
    }

    pub fn get_priority_table(&self) -> impl Future<Item = Vec<PriorityTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let priority_table = Priority.load::<PriorityTable>(&conn).map_err(Into::into);

        future::result(priority_table)
    }

    pub fn get_qualifier_table(&self) -> impl Future<Item = Vec<QualifierTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let qualifier_table = Qualifier.load::<QualifierTable>(&conn).map_err(Into::into);

        future::result(qualifier_table)
    }

    pub fn get_status_table(&self) -> impl Future<Item = Vec<StatusTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let status_table = Status.load::<StatusTable>(&conn).map_err(Into::into);

        future::result(status_table)
    }

    pub fn get_event_by_status(
        &self,
        s_id: i32,
    ) -> impl Future<Item = Vec<EventsTable>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let event = Events
            .filter(schema::Events::dsl::status_id.eq(s_id))
            .load::<EventsTable>(&conn)
            .map_err(Into::into);

        future::result(event)
    }

    pub fn get_signature_by_qualifier(
        &self,
        q_id: i32,
    ) -> impl Future<Item = Vec<String>, Error = Error> {
        let conn = self.pool.get().unwrap();
        let event = Events
            .select(schema::Events::dsl::signature)
            .filter(schema::Events::dsl::qualifier_id.eq(q_id))
            .load::<String>(&conn)
            .map_err(Into::into);

        future::result(event)
    }

    pub fn update_qualifier_id(
        &self,
        id: i32,
        new_qualifier_id: i32,
    ) -> impl Future<Item = usize, Error = Error> {
        let conn = self.pool.get().unwrap();
        DB::get_status_table(&self)
            .and_then(move |status_table| {
                let active = status_table
                    .iter()
                    .find(|x| x.status == "active".to_string());
                let result = Events
                    .or_filter(schema::Events::dsl::event_id.eq(id))
                    .load::<EventsTable>(&conn)
                    .map_err(Into::into)
                    .and_then(|mut event| {
                        event[0].event_id = None;
                        event[0].qualifier_id = new_qualifier_id;
                        event[0].status_id = active.unwrap().status_id.unwrap();
                        event[0].last_modification_time =
                            Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
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
