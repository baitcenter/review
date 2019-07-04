use remake::classification::EventClassifier;
use remake::event::{RawEventDatabase, RawEventRoTransaction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::{BufReader, Write};
use std::path::Path;

use crate::views::bin2str;

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

    pub(crate) fn write_benign_rules_to_file(path: &str, clusters: &[Cluster]) -> io::Result<()> {
        let mut file = File::create(path)?;
        let mut buff = String::new();
        for cluster in clusters {
            if cluster.suspicious == "Benign" {
                buff.push_str(&cluster.signature);
                buff.push_str(&"\n");
            }
        }
        file.write_all(buff.as_bytes())?;
        Ok(())
    }
}

pub(crate) struct ClusterSet {
    pub(crate) clusters: Vec<Cluster>,
}

impl ClusterSet {
    pub(crate) fn from_paths<P: AsRef<Path>>(model: P, clusters: P, raw: P) -> io::Result<Self> {
        let reader = BufReader::new(File::open(model)?);
        let model: EventClassifier = rmp_serde::from_read(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let sigs = model
            .clustering
            .index()
            .iter()
            .map(|(sig, id)| (id, sig))
            .collect::<HashMap<_, _>>();

        let reader = BufReader::new(File::open(clusters)?);
        let cluster_ids: HashMap<usize, HashSet<u64>> = rmp_serde::from_read(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let raw = RawEventDatabase::new(raw.as_ref())?;
        let ro_txn = raw.begin_ro_txn()?;

        let mut clusters = Vec::with_capacity(cluster_ids.len());
        for (id, events) in cluster_ids {
            let cluster = Cluster::with_examples(id, &events, sigs.get(&id).map(|e| *e), &ro_txn)?;
            clusters.push(cluster);
        }
        clusters.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(ClusterSet { clusters })
    }
}
