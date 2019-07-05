use chrono::Utc;
use cursive::view::Position;
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use std::error::Error;
use std::io;
use std::path::Path;
use std::sync::mpsc;

use crate::models::{write_clusters, Cluster};
use crate::views::{ClusterSelectView, MainView};

pub struct ClusterView<'a> {
    cursive: Cursive,
    cl_view_rx: mpsc::Receiver<ClusterViewMessage>,
    cl_view_tx: mpsc::Sender<ClusterViewMessage>,
    cluster_path: &'a str,
}

pub enum ClusterViewMessage {
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

    pub fn run_feedback_mode(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.cl_view_rx.try_iter().next() {
                match message {
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
                                        &view.clusters.clusters,
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

fn delete_old_events(s: &mut Cursive) {
    let mut path = cursive::view::ViewPath::new();
    path.path = vec![0];
    match s
        .call_on_id(
            "cluster_select",
            |v: &mut ClusterSelectView| -> io::Result<()> {
                let clusters = v.clusters.delete_old_events().map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("An error occurred while deleting events: {}", e),
                    )
                })?;
                write_clusters(&v.clusters_path, &clusters).map_err(|e| {
                    io::Error::new(e.kind(), format!("Writing clusters failed: {}", e))
                })?;
                Ok(())
            },
        )
        .unwrap()
    {
        Ok(_) => ClusterView::create_popup_window_then_quit(s, "Deleted successfully."),
        Err(e) => ClusterView::create_popup_window_then_quit(s, &format!("{}", e)),
    }
}
