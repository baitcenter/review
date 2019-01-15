use chrono::Utc;
use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{Position, SizeConstraint};
use cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use cursive::Cursive;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc;

#[derive(Deserialize)]
pub struct Cluster {
    name: String,
    signature: String,
    size: usize,
    suspicious: String,
    examples: Vec<String>,
}

impl Cluster {
    pub fn get_cluster_properties(
        cluster: &Cluster,
        cluster_ids: &BTreeMap<String, Vec<u64>>,
    ) -> String {
        if let Some(id) = cluster_ids.get(&cluster.name) {
            let mut ids = id.to_vec();
            if ids.len() >= 10 {
                let (id_first, _id_last) = ids.split_at(10);
                ids = id_first.to_vec();
            }
            format!(
                "cluster name: {}\nsignature: {}\nqualifier: {}\nsize: {}\nexamples: {:#?}\nIDs: {:#?}\n\n",
                cluster.name, cluster.signature, cluster.suspicious, cluster.size, cluster.examples, ids
            )
        } else {
            format!(
                "cluster name: {}\nsignature: {}\nqualifier: {}\nsize: {}\nexamples: {:#?}\nIDs: No information\n\n",
                cluster.name, cluster.signature, cluster.suspicious, cluster.size, cluster.examples
            )
        }
    }

    pub fn read_cluster_ids_from_json_files<P: AsRef<Path>>(
        path: P,
        clusters: &[Cluster],
    ) -> Result<BTreeMap<String, Vec<u64>>, Box<std::error::Error>> {
        let mut cluster_ids: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader)?;

        for cluster in clusters {
            if let Some(value) = data.get(&cluster.name) {
                let ids: Vec<u64> = serde_json::from_value(value.clone()).unwrap();
                cluster_ids.insert(cluster.name.clone(), ids);
            }
        }

        Ok(cluster_ids)
    }

    pub fn read_clusters_from_json_files<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Cluster>, Box<std::error::Error>> {
        let mut clusters: Vec<Cluster> = Vec::new();
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader)?;

        if let Some(cluster_details) = data.get("Cluster Details") {
            clusters = serde_json::from_value(cluster_details.clone()).unwrap();
        }

        Ok(clusters)
    }

    pub fn check_dir(target_dir: &str) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        let mut json_files: Vec<std::path::PathBuf> = Vec::new();
        for entry in fs::read_dir(target_dir)? {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(extension) = entry.path().extension() {
                            if extension.to_str() == Some("json") {
                                json_files.push(entry.path());
                            }
                        }
                    }
                }
            }
        }

        Ok(json_files)
    }

    pub fn write_benign_rules_to_file(path: &str, clusters: &[Cluster]) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut buff = String::new();
        for cluster in clusters {
            if cluster.suspicious == "Benign".to_string() {
                buff.push_str(&cluster.signature);
                buff.push_str(&"\n");
            }
        }
        file.write_all(buff.as_bytes())?;
        Ok(())
    }
}

pub struct ClusterView {
    cursive: Cursive,
    cl_view_rx: mpsc::Receiver<ClusterViewMessage>,
    cl_view_tx: mpsc::Sender<ClusterViewMessage>,
    clusters: Vec<Cluster>,
    cluster_ids: BTreeMap<String, Vec<u64>>,
    path: String,
}

pub enum ClusterViewMessage {
    PrintClusterProps(usize),
    SaveClusterQualifier((usize, i64)),
    WriteBenignRules(),
}

impl ClusterView {
    pub fn new(dir_paths: &[&str]) -> Result<ClusterView, Box<Error>> {
        let (cl_view_tx, cl_view_rx) = mpsc::channel::<ClusterViewMessage>();
        let mut clusters: Vec<Cluster> = Vec::new();
        let mut cluster_ids: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let mut path = String::new();

        for dir_path in dir_paths.iter() {
            match Cluster::check_dir(&dir_path) {
                Ok(json_files) => {
                    for f in json_files.iter() {
                        if let Ok(mut c) = Cluster::read_clusters_from_json_files(f) {
                            clusters.append(&mut c);
                            if path.is_empty() {
                                path = dir_path.to_string();
                            }
                        }
                    }
                }
                Err(e) => eprintln!("{}: {}", dir_path, e),
            }
        }

        if !clusters.is_empty() {
            for dir_path in dir_paths.iter() {
                match Cluster::check_dir(&dir_path) {
                    Ok(json_files) => {
                        for f in json_files.iter() {
                            if let Ok(mut id) =
                                Cluster::read_cluster_ids_from_json_files(f, &clusters)
                            {
                                cluster_ids.append(&mut id);
                            }
                        }
                    }
                    Err(e) => eprintln!("{}: {}", dir_path, e),
                }
            }
        }

        if clusters.is_empty() {
            eprintln!("Could not find JSON files containing cluster information.");
            std::process::exit(1);
        }

        let mut cluster_view = ClusterView {
            cursive: Cursive::default(),
            cl_view_tx,
            cl_view_rx,
            clusters,
            cluster_ids,
            path,
        };

        let names: Vec<String> = cluster_view
            .clusters
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let mut cluster_select = SelectView::new();
        let index_width = ((names.len() + 1) as f64).log10() as usize + 1;
        for (i, label) in names.iter().enumerate() {
            let index_str = (i + 1).to_string();
            cluster_select.add_item(
                " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
                i,
            );
        }

        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_select.set_on_submit(move |_s, i| {
            cl_view_tx_clone
                .send(ClusterViewMessage::PrintClusterProps(*i))
                .unwrap();
        });

        let quit_view = TextView::new("Press q to exit.".to_string());
        let save_view = TextView::new(
            "Press w to write the signatures of clusters qualified as benign into a file."
                .to_string(),
        );
        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(cluster_select.scrollable().full_width().fixed_height(30))
            .child(DummyView)
            .child(DummyView)
            .child(quit_view)
            .child(save_view);

        let cluster_prop_box1 =
            BoxView::new(SizeConstraint::Full, SizeConstraint::Full, top_layout)
                .with_id("cluster_view");
        let cluster_prop_box2 = BoxView::new(
            SizeConstraint::Fixed(60),
            SizeConstraint::Fixed(60),
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("").with_id("cluster_properties"))
                    .child(
                        Dialog::around(TextView::new(
                            "Please Select a cluster from the left lists",
                        ))
                        .with_id("cluster_properties2"),
                    ),
            ),
        );

        cluster_view.cursive.add_layer(
            LinearLayout::horizontal()
                .child(cluster_prop_box1)
                .child(cluster_prop_box2),
        );

        cluster_view.cursive.add_global_callback('q', |s| s.quit());
        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_view.cursive.add_global_callback('w', move |_s| {
            cl_view_tx_clone
                .send(ClusterViewMessage::WriteBenignRules())
                .unwrap();
        });

        Ok(cluster_view)
    }

    pub fn run(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.cl_view_rx.try_iter().next() {
                match message {
                    ClusterViewMessage::PrintClusterProps(item) => {
                        let mut cluster_prop_window1 = self
                            .cursive
                            .find_id::<TextView>("cluster_properties")
                            .unwrap();
                        cluster_prop_window1.set_content(Cluster::get_cluster_properties(
                            &self.clusters[item],
                            &self.cluster_ids,
                        ));

                        let mut cluster_prop_window2 = self
                            .cursive
                            .find_id::<Dialog>("cluster_properties2")
                            .unwrap();
                        let cl_view_tx_clone = self.cl_view_tx.clone();

                        let mut qualifier_select = SelectView::new();
                        qualifier_select.add_item("Suspicious".to_string(), 1);
                        qualifier_select.add_item("Benign".to_string(), 2);
                        qualifier_select.add_item("None".to_string(), 3);

                        cluster_prop_window2.set_content(qualifier_select.on_submit(
                            move |_s, qualifier: &i64| {
                                cl_view_tx_clone
                                    .send(ClusterViewMessage::SaveClusterQualifier((
                                        item, *qualifier,
                                    )))
                                    .unwrap();
                            },
                        ))
                    }

                    ClusterViewMessage::SaveClusterQualifier(item) => {
                        let qualifier: String;
                        if item.1 == 1 {
                            qualifier = "Suspicious".to_string();
                        } else if item.1 == 2 {
                            qualifier = "Benign".to_string();
                        } else {
                            qualifier = "None".to_string();
                        }

                        if self.clusters[item.0].suspicious != qualifier {
                            self.clusters[item.0].suspicious = qualifier;
                        }
                    }

                    ClusterViewMessage::WriteBenignRules() => {
                        let mut file_path: String = self.path.clone();
                        if self.path.chars().last().unwrap() != '/' {
                            file_path.push_str("/");
                        }
                        file_path.push_str(&Utc::now().format("%Y%m%d%H%M%S").to_string());
                        file_path.push_str("_benign_rules.txt");

                        match Cluster::write_benign_rules_to_file(
                            &file_path.as_str(),
                            &self.clusters,
                        ) {
                            Ok(_) => {
                                let popup_message = format!(
                                    "The benign rules have been saved to \n\n{:?}.",
                                    file_path
                                );
                                ClusterView::create_popup_window(
                                    &mut self.cursive,
                                    popup_message.as_str(),
                                );
                            }
                            Err(e) => {
                                let popup_message = format!(
                                    "Failed to write benign rules to {:?}.\nError: {}",
                                    file_path, e
                                );
                                ClusterView::create_popup_window(
                                    &mut self.cursive,
                                    popup_message.as_str(),
                                );
                            }
                        }
                    }
                }
            }

            self.cursive.step();
        }
    }

    pub fn create_popup_window(cursive: &mut Cursive, popup_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .dismiss_button("OK"),
        );
    }
}
