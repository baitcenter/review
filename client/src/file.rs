use chrono::Utc;
use cursive::view::Position;
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use remake::event::RawEventDatabase;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc;

use crate::models::Cluster;
use crate::views::MainView;

pub struct ClusterView<'a> {
    cursive: Cursive,
    cl_view_rx: mpsc::Receiver<ClusterViewMessage>,
    cl_view_tx: mpsc::Sender<ClusterViewMessage>,
    cluster_path: &'a str,
    raw_db_path: &'a str,
}

pub enum ClusterViewMessage {
    DeleteOldEvents(),
    WriteBenignRules(),
}

impl<'a> ClusterView<'a> {
    pub fn new(
        cluster_path: &'a str,
        model_path: &str,
        raw_db_path: &'a str,
    ) -> Result<ClusterView<'a>, Box<Error>> {
        let (cl_view_tx, cl_view_rx) = mpsc::channel::<ClusterViewMessage>();

        let main_view = MainView::from_paths(model_path, cluster_path, raw_db_path)?;
        let mut cluster_view = ClusterView {
            cursive: Cursive::default(),
            cl_view_tx,
            cl_view_rx,
            cluster_path,
            raw_db_path,
        };

        cluster_view.cursive.add_layer(main_view);
        cluster_view
            .cursive
            .add_global_callback('q', delete_and_quit);
        let cl_view_tx_clone = cluster_view.cl_view_tx.clone();
        cluster_view.cursive.add_global_callback('w', move |_| {
            cl_view_tx_clone
                .send(ClusterViewMessage::WriteBenignRules())
                .unwrap();
        });

        Ok(cluster_view)
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

                        let result = self
                            .cursive
                            .call_on_id(
                                "cluster_select",
                                |view: &mut crate::views::ClusterSelectView| {
                                    Cluster::write_benign_rules_to_file(
                                        &file_path.as_str(),
                                        &view.clusters,
                                    )
                                },
                            )
                            .unwrap();
                        match result {
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

fn delete_and_quit(s: &mut Cursive) {
    s.screen_mut().add_layer_at(
        Position::new(cursive::view::Offset::Center, cursive::view::Offset::Parent(5)),
        Dialog::new()
            .content(TextView::new("Would you like to delete old events?\nThe most recent 25 events will be kept and the rest will be deleted from cluster file and database."))
            .dismiss_button("Back to the previous window")
            .button("No", Cursive::quit)
            .button("Yes", delete_old_events)
    );
}

fn delete_old_events(s: &mut Cursive) {}
