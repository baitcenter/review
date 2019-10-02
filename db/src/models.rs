use super::schema::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "category"]
#[primary_key(id)]
pub struct CategoryTable {
    pub id: i32,
    pub name: String,
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
#[table_name = "cluster"]
#[belongs_to(CategoryTable, foreign_key = "category_id")]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
#[belongs_to(StatusTable, foreign_key = "status_id")]
#[belongs_to(QualifierTable, foreign_key = "qualifier_id")]
pub struct ClustersTable {
    pub id: i32,
    pub cluster_id: Option<String>,
    pub category_id: i32,
    pub detector_id: i32,
    pub event_ids: Option<Vec<u8>>,
    pub raw_event_id: Option<i32>,
    pub qualifier_id: i32,
    pub status_id: i32,
    pub signature: String,
    pub size: String,
    pub score: Option<f64>,
    pub data_source_id: i32,
    pub last_modification_time: Option<chrono::NaiveDateTime>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClusterUpdate {
    pub cluster_id: String,
    pub detector_id: i32,
    pub signature: Option<String>,
    pub score: Option<f64>,
    pub data_source: String,
    pub data_source_type: String,
    pub size: Option<usize>,
    pub event_ids: Option<Vec<u64>>,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "data_source"]
#[primary_key(id)]
pub struct DataSourceTable {
    pub id: i32,
    pub topic_name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Example {
    pub raw_event: String,
    pub event_ids: Vec<u64>,
}

#[derive(Debug, Associations, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "outlier"]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
pub struct OutliersTable {
    pub id: i32,
    pub raw_event: Vec<u8>,
    pub data_source_id: i32,
    pub event_ids: Vec<u8>,
    pub raw_event_id: Option<i32>,
    pub size: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OutlierUpdate {
    pub outlier: Vec<u8>,
    pub data_source: String,
    pub data_source_type: String,
    pub event_ids: Vec<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QualifierUpdate {
    pub cluster_id: String,
    pub data_source: String,
    pub qualifier: String,
}

#[derive(Debug, Deserialize, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "qualifier"]
#[primary_key(id)]
pub struct QualifierTable {
    pub id: i32,
    pub description: String,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "raw_event"]
pub struct RawEventTable {
    pub id: i32,
    pub data: Vec<u8>,
    pub data_source_id: i32,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "status"]
#[primary_key(id)]
pub struct StatusTable {
    pub id: i32,
    pub description: String,
}
