use super::schema::*;
use serde::{Deserialize, Serialize};

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
    pub examples: Option<Vec<Example>>,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "DataSource"]
#[primary_key(data_source_id)]
pub struct DataSourceTable {
    pub data_source_id: i32,
    pub topic_name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize)]
pub struct Example {
    pub id: u64,
    pub raw_event: String,
}

impl Ord for Example {
    fn cmp(&self, other: &Example) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Example {
    fn partial_cmp(&self, other: &Example) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Example {
    fn eq(&self, other: &Example) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Associations, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "Outliers"]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
pub struct OutliersTable {
    pub id: Option<i32>,
    pub raw_event: Vec<u8>,
    pub data_source_id: i32,
    pub event_ids: Option<Vec<u8>>,
    pub size: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutlierUpdate {
    pub outlier: Vec<u8>,
    pub data_source: String,
    pub data_source_type: String,
    pub event_ids: Vec<u64>,
}

#[derive(Queryable, Serialize)]
pub struct PriorityTable {
    pub priority_id: Option<i32>,
    pub priority: String,
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
    pub event_id: String,
    pub raw_event: Vec<u8>,
    pub data_source: String,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "Status"]
#[primary_key(status_id)]
pub struct StatusTable {
    pub status_id: Option<i32>,
    pub status: String,
}
