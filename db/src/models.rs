use super::schema::*;
use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct ActionTable {
    pub action_id: Option<i32>,
    pub action: String,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Category"]
#[primary_key(category_id)]
pub struct CategoryTable {
    pub category_id: Option<i32>,
    pub category: String,
}

#[derive(Debug, Queryable, QueryableByName, Serialize)]
#[table_name = "Events"]
pub struct ClusterExample {
    pub cluster_id: Option<String>,
    pub examples: Option<Vec<u8>>,
}

#[derive(
    Debug,
    AsChangeset,
    Associations,
    Identifiable,
    Insertable,
    Queryable,
    QueryableByName,
    Serialize,
)]
#[table_name = "Events"]
#[primary_key(event_id)]
#[belongs_to(CategoryTable, foreign_key = "category_id")]
#[belongs_to(StatusTable, foreign_key = "status_id")]
#[belongs_to(QualifierTable, foreign_key = "qualifier_id")]
pub struct EventsTable {
    pub event_id: Option<i32>,
    pub cluster_id: Option<String>,
    pub description: Option<String>,
    pub category_id: i32,
    pub detector_id: i32,
    pub examples: Option<Vec<u8>>,
    pub priority_id: i32,
    pub qualifier_id: i32,
    pub status_id: i32,
    pub rules: Option<String>,
    pub signature: String,
    pub size: String,
    pub data_source: String,
    pub last_modification_time: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "Outliers"]
pub struct OutliersTable {
    pub outlier_id: Option<i32>,
    pub outlier_raw_event: Vec<u8>,
    pub outlier_data_source: String,
    pub outlier_event_ids: Option<Vec<u8>>,
    pub outlier_size: Option<String>,
}

#[derive(Queryable, Serialize)]
pub struct PriorityTable {
    pub priority_id: Option<i32>,
    pub priority: String,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Qualifier"]
#[primary_key(qualifier_id)]
pub struct QualifierTable {
    pub qualifier_id: Option<i32>,
    pub qualifier: String,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Status"]
#[primary_key(status_id)]
pub struct StatusTable {
    pub status_id: Option<i32>,
    pub status: String,
}
