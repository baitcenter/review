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

#[derive(Queryable, Serialize)]
pub struct _DetectorsTable {
    pub detector_id: Option<i32>,
    pub detector_type: i32,
    pub description: String,
    pub input: Option<String>,
    pub local_db: Option<String>,
    pub output: String,
    pub status_id: i32,
    pub suspicious_token: Option<String>,
    pub target_detector_id: Option<i32>,
    pub time_regex: Option<String>,
    pub time_format: Option<String>,
    pub last_modification_time: Option<String>,
}

#[derive(AsChangeset, Queryable, Serialize)]
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
    pub last_modification_time: Option<String>,
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
pub struct _ReadyTable {
    pub publish_id: Option<i32>,
    pub action_id: i32,
    pub event_id: Option<i32>,
    pub detector_id: Option<i32>,
    pub time_published: Option<i32>,
}

#[derive(Queryable, Serialize)]
pub struct StatusTable {
    pub status_id: Option<i32>,
    pub status: String,
}
