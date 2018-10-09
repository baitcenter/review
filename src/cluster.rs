extern crate serde;
extern crate serde_json;

use std::error::Error;
use std::fs::File;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct Cluster {
    pub cluster_id: String,
    description: String,
    pub examples: Vec<String>,
    category: String,
    confidence: Option<f64>,
    qualifier: i32,
    signature: String,
    count: u32,
}

pub fn read_clusters_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Cluster>, Box<Error>> {
    let json_file = File::open(path)?;
    let clusters: Vec<Cluster> = serde_json::from_reader(json_file)?;
    Ok(clusters)
}
