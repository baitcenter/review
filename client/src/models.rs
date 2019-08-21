use failure::ResultExt;
use remake::classification::EventClassifier;
use remake::cluster::{Load, PacketPrefixClustering, PrefixClustering, PrefixClusteringAccess};
use remake::event::{RawEventDatabase, RawEventRoTransaction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufReader, Write};

use crate::error::{Error, ErrorKind, InitializeErrorReason};
use crate::views::bin2str;
use std::ops::{Index, IndexMut};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Cluster {
    pub(crate) cluster_id: usize,
    pub(crate) signature: String,
    pub(crate) size: usize,
    pub(crate) suspicious: String,
    pub(crate) examples: Vec<String>,
    pub(crate) event_ids: Vec<u64>,
}

impl Cluster {
    fn with_examples<'a>(
        id: usize,
        events: &HashSet<u64>,
        signature: Option<&Vec<u8>>,
        txn: &RawEventRoTransaction<'a>,
    ) -> io::Result<Self> {
        let signature = signature.map(|e| bin2str(e)).unwrap_or_default();

        let mut examples = Vec::new();
        for &id in events.iter() {
            let example = match txn.get(id) {
                Ok(e) => e,
                Err(e) => {
                    if e.kind() == io::ErrorKind::InvalidInput {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            };
            examples.push(bin2str(example));

            // REview displays at most 3 examples for each cluster.
            if examples.len() >= 3 {
                break;
            }
        }

        Ok(Cluster {
            cluster_id: id,
            signature,
            size: events.len(),
            suspicious: "None".to_string(),
            examples,
            // REview displays at most 10 event IDs for each cluster,
            event_ids: events.iter().take(10).cloned().collect(),
        })
    }

    pub(crate) fn get_cluster_properties(&self) -> String {
        format!(
            "cluster id: {}\nsignature: {}\nqualifier: {}\nsize: {}\nexamples: {:#?}\nevent IDs: {:#?}\n\n",
            self.cluster_id, self.signature, self.suspicious, self.size, self.examples, self.event_ids
        )
    }
}

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

pub(crate) struct ClusterSet {
    pub(crate) clusters: Vec<Cluster>,
    cluster_ids: HashMap<usize, HashSet<u64>>,
    raw_db: RawEventDatabase,
}

impl ClusterSet {
    pub(crate) fn from_paths<P: AsRef<Path> + Display>(
        model: P,
        clusters: P,
        raw: P,
    ) -> io::Result<Self> {
        let sigs = match EventClassifier::<PrefixClustering>::from_path(&model) {
            Ok(model) => read_signatures(model),
            Err(_) => EventClassifier::<PacketPrefixClustering>::from_path(&model)
                .map(read_signatures)
                .map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("invalid model: {}", e))
                })?,
        };

        let reader = BufReader::new(
            File::open(clusters)
                .map_err(|e| io::Error::new(e.kind(), format!("cannot open clusters: {}", e)))?,
        );
        let cluster_ids: HashMap<usize, HashSet<u64>> =
            rmp_serde::from_read(reader).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("invalid model: {}", e))
            })?;

        let raw_db = RawEventDatabase::new(raw.as_ref())
            .map_err(|e| io::Error::new(e.kind(), format!("cannot open event database: {}", e)))?;
        let clusters = read_raw_events(&cluster_ids, sigs, &raw_db)?;

        Ok(ClusterSet {
            clusters,
            cluster_ids,
            raw_db,
        })
    }

    pub(crate) fn delete_old_events(&mut self) -> io::Result<HashMap<usize, HashSet<u64>>> {
        let shrunk_clusters = remake::cluster::delete_old_events(&self.cluster_ids, 25);
        let mut event_ids_to_keep =
            shrunk_clusters
                .iter()
                .fold(Vec::<u64>::new(), |mut ids, (_, set)| {
                    ids.extend(set.iter());
                    ids
                });
        event_ids_to_keep.sort_unstable();
        self.raw_db.shrink_to_fit(&event_ids_to_keep).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to delete events in database: {}", e),
            )
        })?;
        Ok(shrunk_clusters)
    }

    pub(crate) fn write_benign_rules<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        let mut buff = String::new();
        for cluster in &self.clusters {
            if cluster.suspicious == "Benign" {
                buff.push_str(&cluster.signature);
                buff.push_str(&"\n");
            }
        }
        file.write_all(buff.as_bytes())?;
        Ok(())
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.clusters.len()
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

fn read_signatures<T: PrefixClusteringAccess>(
    model: EventClassifier<T>,
) -> HashMap<usize, Vec<u8>> {
    model
        .clustering
        .raw_index()
        .into_iter()
        .map(|(sig, id)| (id, sig))
        .collect::<HashMap<_, _>>()
}

fn read_raw_events(
    ids: &HashMap<usize, HashSet<u64>>,
    sigs: HashMap<usize, Vec<u8>>,
    db: &RawEventDatabase,
) -> io::Result<Vec<Cluster>> {
    let ro_txn = db.begin_ro_txn().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot begin database transaction: {}", e),
        )
    })?;

    let mut clusters = Vec::with_capacity(ids.len());
    for (id, events) in ids {
        let cluster = Cluster::with_examples(*id, events, sigs.get(id), &ro_txn)?;
        clusters.push(cluster);
    }
    clusters.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(clusters)
}

pub(crate) fn write_clusters<P: AsRef<Path>>(
    path: P,
    clusters: &HashMap<usize, HashSet<u64>>,
) -> io::Result<()> {
    let file = fs::File::create(path)?;
    if let Err(e) = clusters.serialize(&mut rmp_serde::Serializer::new(file)) {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    Ok(())
}
