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
    pub status: Option<String>,
}

impl Cluster {
    pub fn get_cluster_properties(&self) -> String {
        format!("cluster_id: {}\ndescription: {}\nexamples: {:?}\ncategory: {}\nconfidence: {:?}\nqualifier: {}\nsignature: {}\ncount: {}\nstatus: {:?} \n\n",&self.cluster_id, &self.description, &self.examples, &self.category, &self.confidence, &self.qualifier, &self.signature, &self.count, &self.status)
    }
}

pub fn read_clusters_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Cluster>, Box<Error>> {
    let json_file = File::open(path)?;
    let clusters: Vec<Cluster> = serde_json::from_reader(json_file)?;
    Ok(clusters)
}
