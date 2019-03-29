use super::schema::Events;
use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct ActionTable {
    pub action_id: Option<i32>,
    pub action: String,
}

#[derive(Queryable, Serialize)]
pub struct CategoryTable {
    pub category_id: Option<i32>,
    pub category: String,
}

#[derive(Debug, Queryable, QueryableByName, Serialize)]
#[table_name = "Events"]
pub struct ClusterExample {
    cluster_id: Option<String>,
    examples: Option<String>,
}

#[derive(Debug, AsChangeset, Insertable, Queryable, Serialize)]
#[table_name = "Events"]
pub struct EventsTable {
    pub event_id: Option<i32>,
    pub cluster_id: Option<String>,
    pub description: Option<String>,
    pub category_id: i32,
    pub detector_id: i32,
    pub examples: Option<String>,
    pub priority_id: i32,
    pub qualifier_id: i32,
    pub status_id: i32,
    pub rules: Option<String>,
    pub signature: String,
    pub last_modification_time: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable, Serialize)]
pub struct PriorityTable {
    pub priority_id: Option<i32>,
    pub priority: String,
}

#[derive(Queryable, Serialize)]
pub struct QualifierTable {
    pub qualifier_id: Option<i32>,
    pub qualifier: String,
}

#[derive(Queryable, Serialize)]
pub struct StatusTable {
    pub status_id: Option<i32>,
    pub status: String,
}
