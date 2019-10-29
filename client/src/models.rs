use failure::ResultExt;
use reviewd::{Example, QualifierTable};
use serde::Deserialize;
use std::collections::HashMap;

use crate::error::{Error, ErrorKind, InitializeErrorReason};
use std::ops::{Index, IndexMut};

#[derive(Deserialize, Debug)]
pub struct Cluster {
    pub(crate) cluster_id: String,
    pub(crate) detector_id: u32,
    pub(crate) qualifier: String,
    pub(crate) status: String,
    pub(crate) category: String,
    pub(crate) signature: String,
    pub(crate) data_source: String,
    pub(crate) size: usize,
    pub(crate) score: Option<String>,
    pub(crate) examples: Example,
    pub(crate) last_modification_time: Option<String>,
}

impl Cluster {
    pub(crate) fn get_cluster_properties(&self) -> String {
        let resp = format!("cluster_id: {}\ndetector_id: {}\nqualifier: {}\nstatus: {}\nsignature: {}\ndata_source: {}\n", 
            self.cluster_id,
            self.detector_id,
            self.qualifier,
            self.status,
            self.signature,
            self.data_source,
        );
        let resp = if let Some(score) = &self.score {
            format!(
                "{}score: {}\nsize: {}\nexample: {:#?}\n",
                resp, score, self.size, self.examples
            )
        } else {
            format!(
                "{}score: -\nsize: {}\nexample: {:#?}\n",
                resp, self.size, self.examples
            )
        };
        if let Some(last_modification_time) = &self.last_modification_time {
            format!("{}last_modification_time: {}", resp, last_modification_time)
        } else {
            format!("{}last_modification_time: -", resp)
        }
    }
}

pub(crate) struct ClusterSet {
    pub(crate) clusters: Vec<Cluster>,
    pub(crate) qualifier: HashMap<i32, String>,
    pub(crate) updated_clusters: HashMap<String, usize>,
    pub(crate) url: String,
}

impl ClusterSet {
    pub(crate) fn from_reviewd(url: &str) -> Result<Self, Error> {
        let url = url.trim_end_matches('/');
        let cluster_url = format!(
            r#"{}/api/cluster/search?filter={{"status":["pending review"]}}"#,
            url
        );
        let mut cluster_resp = reqwest::get(cluster_url.as_str())
            .context(ErrorKind::Initialize(InitializeErrorReason::Reqwest))?;
        let clusters = cluster_resp
            .json::<Vec<Cluster>>()
            .context(ErrorKind::Initialize(
                InitializeErrorReason::UnexpectedResponse,
            ))?;
        if clusters.is_empty() {
            return Err(ErrorKind::Initialize(InitializeErrorReason::EmptyCluster).into());
        }

        let qualifier_url = format!("{}/api/qualifier", url);
        let mut qualifier_resp = reqwest::get(qualifier_url.as_str())
            .context(ErrorKind::Initialize(InitializeErrorReason::Reqwest))?;
        let qualifier = qualifier_resp
            .json::<Vec<QualifierTable>>()
            .context(ErrorKind::Initialize(
                InitializeErrorReason::UnexpectedResponse,
            ))?
            .iter()
            .map(|q| (q.id, q.description.clone()))
            .collect();

        Ok(Self {
            clusters,
            qualifier,
            updated_clusters: HashMap::<String, usize>::default(),
            url: url.to_string(),
        })
    }
}

impl Index<usize> for ClusterSet {
    type Output = Cluster;

    fn index(&self, i: usize) -> &Self::Output {
        &self.clusters[i]
    }
}

impl IndexMut<usize> for ClusterSet {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.clusters[i]
    }
}
