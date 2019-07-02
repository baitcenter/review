use chrono::Utc;
use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{Position, SizeConstraint, ViewWrapper};
use cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use cursive::Cursive;
use remake::classification::EventClassifier;
use remake::event::RawEventDatabase;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use std::error::Error;
use std::sync::mpsc;

#[derive(Deserialize, Serialize)]
pub struct Cluster {
    cluster_id: usize,
    signature: String,
    size: usize,
    suspicious: String,
    examples: Vec<String>,
    event_ids: Vec<u64>,
}

impl Cluster {
    pub fn get_cluster_properties(cluster: &Cluster) -> String {
        format!(
            "cluster id: {}\nsignature: {}\nqualifier: {}\nsize: {}\nexamples: {:#?}\nevent IDs: {:#?}\n\n",
            cluster.cluster_id, cluster.signature, cluster.suspicious, cluster.size, cluster.examples, cluster.event_ids
        )
    }

    pub fn write_benign_rules_to_file(path: &str, clusters: &[Cluster]) -> std::io::Result<()> {
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

pub struct ClusterView<'a> {
    cursive: Cursive,
    cl_view_rx: mpsc::Receiver<ClusterViewMessage>,
    cl_view_tx: mpsc::Sender<ClusterViewMessage>,
    clusters: Vec<Cluster>,
    cluster_path: &'a str,
    raw_db_path: &'a str,
}

pub enum ClusterViewMessage {
    DeleteOldEvents(),
    PopupQuitWindows(),
    PrintClusterProps(usize),
    SaveClusterQualifier((usize, i64)),
    WriteBenignRules(),
}

impl<'a> ClusterView<'a> {
    pub fn new(
        cluster_path: &'a str,
        model_path: &str,
        raw_db_path: &'a str,
    ) -> Result<ClusterView<'a>, Box<Error>> {
        let (cl_view_tx, cl_view_rx) = mpsc::channel::<ClusterViewMessage>();
        let mut clusters: Vec<Cluster> = Vec::new();

        let model = match ClusterView::read_model_file(model_path) {
            Ok(model) => model,
            Err(e) => {
                eprintln!("Model file {} could not be opened: {}", model_path, e);
                std::process::exit(1);
            }
        };
        let index = model.clustering.index();
        let sigs = index
            .iter()
            .map(|(sig, id)| (id, sig))
            .collect::<HashMap<_, _>>();
        let cls = match ClusterView::read_clusters_file(cluster_path) {
            Ok(cls) => cls,
            Err(e) => {
                eprintln!("Cluster file {} could not be opened: {}", cluster_path, e);
                std::process::exit(1);
            }
        };
        let raw_event_db = match RawEventDatabase::new(&Path::new(&raw_db_path)) {
            Ok(raw_event_db) => raw_event_db,
            Err(e) => {
                eprintln!("Raw database {} could not be opened: {}", raw_db_path, e);
                std::process::exit(1);
            }
        };
        let ro_txn = match raw_event_db.begin_ro_txn() {
            Ok(ro_txn) => ro_txn,
            Err(e) => {
                eprintln!("An error occurs while accessing raw database: {}", e);
                std::process::exit(1);
            }
        };

        for (cluster_id, events) in cls.iter() {
            let signature = if let Some(sig) = sigs.get(cluster_id) {
                if let Ok(sig) = std::str::from_utf8(&sig) {
                    sig.to_string()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };

            // REview displays at most 10 event ids for each cluster
            let event_ids = events.iter().take(10).cloned().collect::<Vec<_>>();
            let mut examples = event_ids
                .iter()
                .map(|event_id| ro_txn.get(*event_id))
                .filter_map(Result::ok)
                .map(|event| std::str::from_utf8(event))
                .filter_map(Result::ok)
                .map(|event| {
                    // if the length of an example is longer than 500,
                    // REview only uses first 500
                    if event.len() > 500 {
                        event[..500].to_string()
                    } else {
                        event.to_string()
                    }
                })
                .collect::<Vec<_>>();

            let min_example_count = std::cmp::min(events.len(), 3);
            while examples.len() < min_example_count {
                examples.push("<undecodable>".to_string());
            }

            let size = events.len();
            // REview displays at most 3 examples for each cluster
            if examples.len() >= 4 {
                let (examples, _) = examples.split_at(3);
                let cluster = Cluster {
                    cluster_id: *cluster_id,
                    signature,
                    size,
                    suspicious: "None".to_string(),
                    examples: examples.to_vec(),
                    event_ids,
                };
                clusters.push(cluster);
            } else {
                let cluster = Cluster {
                    cluster_id: *cluster_id,
                    signature,
                    size,
                    suspicious: "None".to_string(),
                    examples,
                    event_ids,
                };
                clusters.push(cluster);
            }
        }
        clusters.sort_by(|a, b| b.size.cmp(&a.size));

        let mut cluster_view = ClusterView {
            cursive: Cursive::default(),
            cl_view_tx,
            cl_view_rx,
            clusters,
            cluster_path,
            raw_db_path,
        };

        let names: Vec<String> = cluster_view
            .clusters
            .iter()
            .map(|c| c.signature.clone())
            .collect();

        let mut cluster_select = crate::views::ClusterSelectView::new();
        let index_width = ((names.len() + 1) as f64).log10() as usize + 1;
        for (i, label) in names.iter().enumerate() {
            let index_str = (i + 1).to_string();
            cluster_select.with_view_mut(|v| v.add_item(
                " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
                i,
            ));
        }

        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_select.with_view_mut(|v| v.set_on_submit(move |_, i| {
            cl_view_tx_clone
                .send(ClusterViewMessage::PrintClusterProps(*i))
                .unwrap();
        }));

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
                .child(cluster_prop_box2)
                .scrollable(),
        );

        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_view.cursive.add_global_callback('q', move |_| {
            cl_view_tx_clone
                .send(ClusterViewMessage::PopupQuitWindows())
                .unwrap();
        });
        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_view.cursive.add_global_callback('w', move |_| {
            cl_view_tx_clone
                .send(ClusterViewMessage::WriteBenignRules())
                .unwrap();
        });

        Ok(cluster_view)
    }

    fn read_model_file<P: AsRef<Path>>(path: P) -> Result<EventClassifier, Box<std::error::Error>> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let classifier: EventClassifier = rmp_serde::from_read(reader)?;

        Ok(classifier)
    }

    fn read_clusters_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<usize, HashSet<u64>>, Box<std::error::Error>> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let cluster: HashMap<usize, HashSet<u64>> = rmp_serde::from_read(reader)?;

        Ok(cluster)
    }

    fn write_clusters_file(
        path: &str,
        clusters: &HashMap<usize, HashSet<u64>>,
    ) -> std::io::Result<()> {
        let file = fs::File::create(path)?;
        if let Err(e) = clusters.serialize(&mut rmp_serde::Serializer::new(file)) {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
        Ok(())
    }

    pub fn run_feedback_mode(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.cl_view_rx.try_iter().next() {
                match message {
                    ClusterViewMessage::DeleteOldEvents() => {
                        let cls = match ClusterView::read_clusters_file(self.cluster_path) {
                            Ok(cls) => cls,
                            Err(e) => {
                                let err_msg = format!(
                                    "Cluster file {} could not be opened: {}",
                                    self.cluster_path, e
                                );
                                ClusterView::create_popup_window_then_quit(
                                    &mut self.cursive,
                                    err_msg.as_str(),
                                );
                                // If we use unreachable or panic here,
                                // Cursive stops working without displaying
                                // above err_msg
                                HashMap::<usize, HashSet<u64>>::new()
                            }
                        };
                        if let Ok(mut raw_event_db) =
                            RawEventDatabase::new(&Path::new(self.raw_db_path))
                        {
                            let shrunken_clusters = remake::cluster::delete_old_events(&cls, 25);
                            let mut event_ids_to_keep =
                                shrunken_clusters
                                    .iter()
                                    .fold(Vec::new(), |mut ids, (_, set)| {
                                        ids.extend(set.iter());
                                        ids
                                    });
                            event_ids_to_keep.sort_unstable();
                            match raw_event_db.shrink_to_fit(&event_ids_to_keep) {
                                Ok(_) => {
                                    match ClusterView::write_clusters_file(&self.cluster_path, &cls)
                                    {
                                        Ok(_) => {
                                            ClusterView::create_popup_window_then_quit(
                                                &mut self.cursive,
                                                "Old events has been successfully deleted.",
                                            );
                                        }
                                        Err(e) => {
                                            let err_msg = format!(
                                                "An error occurs while writing clusters to a file: {}",
                                                e
                                            );
                                            ClusterView::create_popup_window_then_quit(
                                                &mut self.cursive,
                                                err_msg.as_str(),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_msg =
                                        format!("An error occurs while deleting events: {}", e);
                                    ClusterView::create_popup_window_then_quit(
                                        &mut self.cursive,
                                        err_msg.as_str(),
                                    );
                                }
                            };
                        } else {
                            let err_msg =
                                format!("Raw database {} could not be opened", self.raw_db_path);
                            ClusterView::create_popup_window_then_quit(
                                &mut self.cursive,
                                err_msg.as_str(),
                            );
                        }
                    }
                    ClusterViewMessage::PopupQuitWindows() => {
                        let cl_view_tx_clone = self.cl_view_tx.clone();
                        self.cursive.screen_mut().add_layer_at(
                            Position::new(
                                cursive::view::Offset::Center,
                                cursive::view::Offset::Parent(5),
                            ),
                            Dialog::new()
                                .content(TextView::new("Would you like to delete old events?\nThe most recent 25 events will be kept and the rest will be deleted from cluster file and database."))
                                .dismiss_button("Back to the previous window")
                                .button("No", Cursive::quit)
                                .button("Yes", move |_| {
                                    cl_view_tx_clone
                                        .send(ClusterViewMessage::DeleteOldEvents())
                                        .unwrap();
                                })
                        );
                    }

                    ClusterViewMessage::PrintClusterProps(item) => {
                        let mut cluster_prop_window1 = self
                            .cursive
                            .find_id::<TextView>("cluster_properties")
                            .unwrap();
                        cluster_prop_window1
                            .set_content(Cluster::get_cluster_properties(&self.clusters[item]));

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
                            move |_, qualifier: &i64| {
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
                        let mut file_path = match Path::new(self.cluster_path).parent() {
                            Some(path) => match path.to_str() {
                                Some(path) => path.to_string(),
                                None => {
                                    let err_msg = format!(
                                        "An error occurs while parsing {}",
                                        self.cluster_path
                                    );
                                    ClusterView::create_popup_window_then_quit(
                                        &mut self.cursive,
                                        err_msg.as_str(),
                                    );
                                    // If we use unreachable or panic here,
                                    // Cursive stops working without displaying
                                    // above err_msg
                                    String::new()
                                }
                            },
                            None => {
                                let err_msg =
                                    format!("An error occurs while parsing {}", self.cluster_path);
                                ClusterView::create_popup_window_then_quit(
                                    &mut self.cursive,
                                    err_msg.as_str(),
                                );
                                // If we use unreachable or panic here,
                                // Cursive stops working without displaying
                                // above err_msg
                                String::new()
                            }
                        };
                        if !file_path.ends_with('/') {
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
                                    "An error occurs while writing benign rules to {:?}.\nError: {}",
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

    fn create_popup_window(cursive: &mut Cursive, popup_message: &str) {
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

    fn create_popup_window_then_quit(cursive: &mut Cursive, popup_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .button("OK", Cursive::quit),
        );
    }
}
