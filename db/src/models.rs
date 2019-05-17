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

#[derive(Debug, Queryable, QueryableByName, Serialize)]
#[table_name = "Clusters"]
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
#[table_name = "Clusters"]
#[belongs_to(CategoryTable, foreign_key = "category_id")]
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
    pub data_source: String,
    pub last_modification_time: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterUpdate {
    pub cluster_id: String,
    pub detector_id: i32,
    pub signature: Option<String>,
    pub data_source: String,
    pub size: Option<usize>,
    pub examples: Option<Vec<Example>>,
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

#[derive(Debug, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "Outliers"]
pub struct OutliersTable {
    pub outlier_id: Option<i32>,
    pub outlier_raw_event: Vec<u8>,
    pub outlier_data_source: String,
    pub outlier_event_ids: Option<Vec<u8>>,
    pub outlier_size: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutlierUpdate {
    pub outlier: Vec<u8>,
    pub data_source: String,
    pub event_ids: Vec<u64>,
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
