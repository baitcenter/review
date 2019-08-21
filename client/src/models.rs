use failure::ResultExt;
use serde::Deserialize;
use std::collections::HashMap;

use crate::error::{Error, ErrorKind, InitializeErrorReason};
use std::ops::{Index, IndexMut};

#[derive(Deserialize, Debug)]
pub struct ClusterForHttpClientMode {
    pub(crate) cluster_id: String,
    pub(crate) detector_id: u32,
    pub(crate) qualifier: String,
    pub(crate) status: String,
    pub(crate) category: String,
    pub(crate) signature: String,
    pub(crate) data_source: String,
    pub(crate) size: usize,
    pub(crate) score: String,
    pub(crate) examples: Vec<db::models::Example>,
    pub(crate) last_modification_time: String,
}

impl ClusterForHttpClientMode {
    pub(crate) fn get_cluster_properties(&self) -> String {
        // REview displays at most 3 examples per cluster
        // if the length of an example is longer than 500,
        // REview only uses first 500
        let examples = if self.examples.len() > 3 {
            let (examples, _) = self.examples.split_at(3);
            examples
                .iter()
                .map(|e| {
                    if e.raw_event.len() > 500 {
                        db::models::Example {
                            id: e.id,
                            raw_event: e.raw_event[..500].to_string(),
                        }
                    } else {
                        e.clone()
                    }
                })
                .collect::<Vec<_>>()
        } else {
            self.examples
                .iter()
                .map(|e| {
                    if e.raw_event.len() > 500 {
                        db::models::Example {
                            id: e.id,
                            raw_event: e.raw_event[..500].to_string(),
                        }
                    } else {
                        e.clone()
                    }
                })
                .collect::<Vec<_>>()
        };
        format!("cluster_id: {}\ndetector_id: {}\nqualifier: {}\nstatus: {}\nsignature: {}\ndata_source: {}\nscore: {}\nsize: {}\nexample: {:#?}\nlast_modification_time: {}", 
            self.cluster_id,
            self.detector_id,
            self.qualifier,
            self.status,
            self.signature,
            self.data_source,
            self.score,
            self.size,
            examples,
            self.last_modification_time,
        )
    }
}

pub(crate) struct ClusterSetForHttpClientMode {
    pub(crate) clusters: Vec<ClusterForHttpClientMode>,
    pub(crate) qualifier: HashMap<i32, String>,
    pub(crate) updated_clusters: HashMap<String, usize>,
    pub(crate) url: String,
}

impl ClusterSetForHttpClientMode {
    pub(crate) fn from_reviewd(url: &str) -> Result<Self, Error> {
        let url = url.trim_end_matches('/');
        let cluster_url = format!(
            r#"{}/api/cluster/search?filter={{"status":["pending review"]}}"#,
            url
        );
        let mut cluster_resp = reqwest::get(cluster_url.as_str())
            .context(ErrorKind::Initialize(InitializeErrorReason::Reqwest))?;
        let clusters = cluster_resp
            .json::<Vec<ClusterForHttpClientMode>>()
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
            .json::<Vec<db::models::QualifierTable>>()
            .context(ErrorKind::Initialize(
                InitializeErrorReason::UnexpectedResponse,
            ))?
            .iter()
            .map(|q| (q.qualifier_id.unwrap(), q.qualifier.clone()))
            .collect();

        Ok(ClusterSetForHttpClientMode {
            clusters,
            qualifier,
            updated_clusters: HashMap::<String, usize>::default(),
            url: url.to_string(),
        })
    }
}

impl Index<usize> for ClusterSetForHttpClientMode {
    type Output = ClusterForHttpClientMode;

    fn index(&self, i: usize) -> &Self::Output {
        &self.clusters[i]
    }
}

impl IndexMut<usize> for ClusterSetForHttpClientMode {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.clusters[i]
    }
}
