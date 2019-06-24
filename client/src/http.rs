use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{Position, SizeConstraint};
use cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use cursive::Cursive;
use serde::Deserialize;
use std::collections::HashMap;

use std::error::Error;
use std::sync::mpsc;

#[derive(Deserialize, Debug)]
pub struct Cluster {
    cluster_id: String,
    detector_id: u32,
    qualifier: String,
    status: String,
    category: String,
    signature: String,
    data_source: String,
    size: usize,
    examples: Vec<db::models::Example>,
    last_modification_time: String,
}

#[derive(Deserialize, Debug)]
pub struct QualifierTable {
    pub qualifier_id: Option<u32>,
    pub qualifier: String,
}

#[derive(Deserialize, Debug)]
pub struct StatusTable {
    pub status_id: Option<u32>,
    pub status: String,
}

pub enum ClusterViewMessage {
    PrintClusterProps(usize),
    SaveClusterQualifier((usize, u32)),
    SendUpdateRequest(),
}

pub struct ClusterView<'a> {
    cursive: Cursive,
    cluster_view_rx: mpsc::Receiver<ClusterViewMessage>,
    cluster_view_tx: mpsc::Sender<ClusterViewMessage>,
    clusters: Vec<Cluster>,
    qualifier: HashMap<u32, String>,
    is_cluster_updated: HashMap<String, u32>,
    url: &'a str,
}

impl<'a> ClusterView<'a> {
    pub fn new(url: &str) -> Result<ClusterView, Box<Error>> {
        let (cluster_view_tx, cluster_view_rx) = mpsc::channel::<ClusterViewMessage>();

        let url = url.trim_end_matches('/');
        let qualifier_url = format!("{}/api/qualifier", url);
        let status_url = format!("{}/api/status", url);

        let status: HashMap<_, _> = match reqwest::get(status_url.as_str()) {
            Ok(mut resp) => match resp.json() as Result<Vec<StatusTable>, reqwest::Error> {
                Ok(data) => data
                    .iter()
                    .map(|s| (s.status_id.unwrap(), s.status.clone()))
                    .collect(),
                Err(e) => {
                    eprintln!(
                        "Unexpected response from server. cannot deserialize status table. {}",
                        e
                    );
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        };
        let qualifier: HashMap<_, _> = match reqwest::get(qualifier_url.as_str()) {
            Ok(mut resp) => {
                match resp.json() as Result<Vec<QualifierTable>, reqwest::Error> {
                    Ok(data) => data
                        .iter()
                        .map(|q| (q.qualifier_id.unwrap(), q.qualifier.clone()))
                        .collect(),
                    Err(e) => {
                        eprintln!("Unexpected response from server. cannot deserialize qualifier table. {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        };
        let review_id = status
            .iter()
            .find(|&x| x.1.to_lowercase() == "pending review")
            .unwrap()
            .0;
        let cluster_url = format!(r#"{}/api/cluster/search?filter={{"status":["pending review"]}}"#, url);
        let clusters = match reqwest::get(cluster_url.as_str()) {
            Ok(mut resp) => match resp.json() as Result<Vec<Cluster>, reqwest::Error> {
                Ok(data) => data,
                Err(e) => {
                    eprintln!(
                        "Unexpected response from server. cannot deserialize Clusters table. {}",
                        e
                    );
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        };
        if clusters.is_empty() {
            eprintln!("Cluster with review status was not found.");
            std::process::exit(1);
        }
        let is_cluster_updated: HashMap<String, u32> = HashMap::new();
        let mut cluster_view = ClusterView {
            cursive: Cursive::default(),
            cluster_view_tx,
            cluster_view_rx,
            clusters,
            qualifier,
            is_cluster_updated,
            url,
        };

        let names: Vec<String> = cluster_view
            .clusters
            .iter()
            .map(|e| e.cluster_id.clone())
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

        let cluster_view_tx_clone = cluster_view.cluster_view_tx.clone();
        cluster_select.set_on_submit(move |_, i| {
            cluster_view_tx_clone
                .send(ClusterViewMessage::PrintClusterProps(*i))
                .unwrap();
        });

        let quit_view = TextView::new("Press q to exit".to_string());
        let save_view = TextView::new("Press s to save changes.".to_string());
        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(cluster_select.scrollable().full_width().fixed_height(20))
            .child(DummyView)
            .child(DummyView)
            .child(save_view)
            .child(quit_view);

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

        cluster_view.cursive.add_global_callback('q', Cursive::quit);
        let cluster_view_tx_clone = cluster_view.cluster_view_tx.clone();
        cluster_view.cursive.add_global_callback('s', move |_| {
            cluster_view_tx_clone
                .send(ClusterViewMessage::SendUpdateRequest())
                .unwrap();
        });

        Ok(cluster_view)
    }
    pub fn run(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.cluster_view_rx.try_iter().next() {
                match message {
                    ClusterViewMessage::PrintClusterProps(item) => {
                        let mut cluster_prop_window1 = self
                            .cursive
                            .find_id::<TextView>("cluster_properties")
                            .unwrap();
                        // REview displays at most 3 examples for each cluster
                        // if the length of an example is longer than 500,
                        // REview only uses first 500
                        let examples = if self.clusters[item].examples.len() > 3 {
                            let (examples, _) = &self.clusters[item].examples.split_at(3);
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
                            self.clusters[item]
                                .examples
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
                        let msg = format!("cluster_id: {}\ndetector_id: {}\nqualifier: {}\nstatus: {}\nsignature: {}\ndata_source: {}\nsize: {}\nexample: {:#?}\nlast_modification_time: {}", 
                            &self.clusters[item].cluster_id,
                            &self.clusters[item].detector_id,
                            &self.clusters[item].qualifier,
                            &self.clusters[item].status,
                            &self.clusters[item].signature,
                            &self.clusters[item].data_source,
                            &self.clusters[item].size,
                            &examples,
                            &self.clusters[item].last_modification_time,
                        );
                        cluster_prop_window1.set_content(msg);

                        let mut cluster_prop_window2 = self
                            .cursive
                            .find_id::<Dialog>("cluster_properties2")
                            .unwrap();
                        let cluster_view_tx_clone = self.cluster_view_tx.clone();

                        let mut qualifier_select = SelectView::new();
                        for (i, qualifier) in &self.qualifier {
                            qualifier_select.add_item(qualifier.to_string(), *i);
                        }
                        cluster_prop_window2.set_content(qualifier_select.on_submit(
                            move |_, qualifier: &u32| {
                                cluster_view_tx_clone
                                    .send(ClusterViewMessage::SaveClusterQualifier((
                                        item, *qualifier,
                                    )))
                                    .unwrap();
                            },
                        ))
                    }

                    ClusterViewMessage::SaveClusterQualifier(item) => {
                        if self.clusters[item.0].qualifier != self.qualifier[&item.1] {
                            self.clusters[item.0].qualifier = self.qualifier[&item.1].clone();
                            self.is_cluster_updated
                                .insert(self.clusters[item.0].cluster_id.clone(), item.1);
                        }
                    }

                    ClusterViewMessage::SendUpdateRequest() => {
                        let mut resp_err: Vec<String> = Vec::new();
                        let mut send_err: Vec<String> = Vec::new();
                        for (cluster_id, qualifier_id) in &self.is_cluster_updated {
                            let url = format!(
                                "{}/api/cluster?cluster_id={}&qualifier_id={}",
                                self.url, cluster_id, qualifier_id
                            );
                            let client = reqwest::Client::new();
                            match client.put(url.as_str()).send() {
                                Ok(resp) => {
                                    if !resp.status().is_success() {
                                        resp_err.push(cluster_id.clone());
                                    }
                                }
                                Err(_) => send_err.push(cluster_id.clone()),
                            }
                        }
                        if resp_err.is_empty() && send_err.is_empty() {
                            let popup_message = "Your changes have been successfully saved.";
                            ClusterView::create_popup_window(
                                &mut self.cursive,
                                popup_message,
                                &"OK",
                            );
                        } else if !resp_err.is_empty() {
                            let popup_message = format!(
                                "Failed to update the following cluster_id:\n\n{:?}",
                                resp_err
                            );
                            ClusterView::create_popup_window(
                                &mut self.cursive,
                                popup_message.as_str(),
                                &"OK",
                            );
                        } else if !send_err.is_empty() {
                            let popup_message = format!("Failed to send update requests of the following cluster_id to backend server:\n\n{:?}", send_err);
                            ClusterView::create_popup_window(
                                &mut self.cursive,
                                popup_message.as_str(),
                                &"OK",
                            );
                        }
                    }
                }
            }

            self.cursive.step();
        }
    }

    fn create_popup_window(cursive: &mut Cursive, popup_message: &str, button_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .dismiss_button(button_message),
        );
    }
}
