use super::schema::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Category"]
#[primary_key(category_id)]
pub struct CategoryTable {
    pub category_id: Option<i32>,
    pub category: String,
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
#[table_name = "Clusters"]
#[belongs_to(CategoryTable, foreign_key = "category_id")]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
#[belongs_to(StatusTable, foreign_key = "status_id")]
#[belongs_to(QualifierTable, foreign_key = "qualifier_id")]
pub struct ClustersTable {
    pub id: Option<i32>,
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

#[derive(Debug, Deserialize, Serialize)]
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
#[table_name = "DataSource"]
#[primary_key(data_source_id)]
pub struct DataSourceTable {
    pub data_source_id: i32,
    pub topic_name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Example {
    pub raw_event: String,
    pub event_ids: Vec<u64>,
}

#[derive(Debug, Associations, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "Outliers"]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
pub struct OutliersTable {
    pub id: Option<i32>,
    pub raw_event: Vec<u8>,
    pub data_source_id: i32,
    pub event_ids: Option<Vec<u8>>,
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
#[table_name = "Qualifier"]
#[primary_key(qualifier_id)]
pub struct QualifierTable {
    pub qualifier_id: Option<i32>,
    pub qualifier: String,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "RawEvent"]
pub struct RawEventTable {
    pub raw_event_id: i32,
    pub raw_event: Vec<u8>,
    pub data_source_id: i32,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Status"]
#[primary_key(status_id)]
pub struct StatusTable {
    pub status_id: Option<i32>,
    pub status: String,
}
