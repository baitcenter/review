use chrono::Utc;
use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{Position, SizeConstraint};
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

#[derive(Deserialize, Debug)]
pub struct Event {
    event_id: u32,
    description: Option<String>,
    cluster_id: Option<String>,
    rules: Option<String>,
    category_id: u32,
    detector_id: u32,
    examples: Option<String>,
    priority_id: u32,
    qualifier_id: u32,
    status_id: u32,
    signature: String,
}

impl Event {
    pub fn get_event_properties(
        &self,
        priority: &HashMap<u32, String>,
        qualifier: &HashMap<u32, String>,
        category: &HashMap<u32, String>,
    ) -> String {
        format!(
            "event_id: {}\ncluster_id: {}\nrules: {}\ndescription: {}\ncategory_id: {}\ndetector_id: {}\nexamples: {}\npriority_id: {}\nqualifier_id: {}\nsignature: {}\n\n",
            &self.event_id,
            &self.cluster_id.as_ref().unwrap_or(&"-".to_string()),
            &self.rules.as_ref().unwrap_or(&"-".to_string()),
            &self.description.as_ref().unwrap_or(&"-".to_string()),
            category.get(&self.category_id).unwrap(),
            &self.detector_id,
            &self.examples.as_ref().unwrap_or(&"-".to_string()),
            priority.get(&self.priority_id).unwrap(),
            qualifier.get(&self.qualifier_id).unwrap(),
            &self.signature,
        )
    }
}

pub enum EventViewMessage {
    PrintEventProps(usize),
    SaveEventQualifier((usize, u32)),
    SendUpdateRequest(),
}

#[derive(Deserialize, Debug)]
pub struct ActionTable {
    pub action_id: Option<u32>,
    pub action: String,
}

#[derive(Deserialize, Debug)]
pub struct CategoryTable {
    pub category_id: Option<u32>,
    pub category: String,
}

#[derive(Deserialize, Debug)]
pub struct PriorityTable {
    pub priority_id: Option<u32>,
    pub priority: String,
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

pub struct EventView<'a> {
    cursive: Cursive,
    event_view_rx: mpsc::Receiver<EventViewMessage>,
    event_view_tx: mpsc::Sender<EventViewMessage>,
    events: Vec<Event>,
    priority: HashMap<u32, String>,
    qualifier: HashMap<u32, String>,
    category: HashMap<u32, String>,
    is_event_updated: HashMap<u32, u32>,
    url: &'a str,
}

impl<'a> EventView<'a> {
    pub fn new(url: &str) -> Result<EventView, Box<Error>> {
        let (event_view_tx, event_view_rx) = mpsc::channel::<EventViewMessage>();

        let mut category: HashMap<u32, String> = HashMap::new();
        let mut priority: HashMap<u32, String> = HashMap::new();
        let mut qualifier: HashMap<u32, String> = HashMap::new();
        let mut status: HashMap<u32, String> = HashMap::new();
        let mut events: Vec<Event> = Vec::new();
        let is_event_updated: HashMap<u32, u32> = HashMap::new();
        let url = url.trim_end_matches('/');

        let category_url = format!("{}/api/category", url);
        let priority_url = format!("{}/api/priority", url);
        let qualifier_url = format!("{}/api/qualifier", url);
        let status_url = format!("{}/api/status", url);

        match reqwest::get(category_url.as_str()) {
            Ok(mut resp) => {
                match resp.json() as Result<Vec<CategoryTable>, reqwest::Error> {
                    Ok(data) => {
                        for d in data {
                            category.insert(d.category_id.unwrap(), d.category);
                        }
                    }
                    Err(e) => {
                        eprintln!("Unexpected response from server. cannot deserialize category table. {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        }
        match reqwest::get(priority_url.as_str()) {
            Ok(mut resp) => {
                match resp.json() as Result<Vec<PriorityTable>, reqwest::Error> {
                    Ok(data) => {
                        for d in data {
                            priority.insert(d.priority_id.unwrap(), d.priority);
                        }
                    }
                    Err(e) => {
                        eprintln!("Unexpected response from server. cannot deserialize priority table. {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        }
        match reqwest::get(qualifier_url.as_str()) {
            Ok(mut resp) => {
                match resp.json() as Result<Vec<QualifierTable>, reqwest::Error> {
                    Ok(data) => {
                        for d in data {
                            qualifier.insert(d.qualifier_id.unwrap(), d.qualifier);
                        }
                    }
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
        }
        match reqwest::get(status_url.as_str()) {
            Ok(mut resp) => match resp.json() as Result<Vec<StatusTable>, reqwest::Error> {
                Ok(data) => {
                    for d in data {
                        status.insert(d.status_id.unwrap(), d.status);
                    }
                }
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
        }

        let review_id = status
            .iter()
            .find(|&x| x.1.to_lowercase() == "review")
            .unwrap()
            .0;
        let event_url = format!("{}/api/event?status_id={}", url, review_id);
        match reqwest::get(event_url.as_str()) {
            Ok(mut resp) => match resp.json() as Result<Vec<Event>, reqwest::Error> {
                Ok(data) => {
                    for d in data {
                        events.push(d);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Unexpected response from server. cannot deserialize Events table. {}",
                        e
                    );
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Err: {}", e);
                std::process::exit(1);
            }
        }
        if events.is_empty() {
            eprintln!("Event with review status was not found.");
            std::process::exit(1);
        }

        let mut event_view = EventView {
            cursive: Cursive::default(),
            event_view_tx,
            event_view_rx,
            events,
            priority,
            qualifier,
            category,
            is_event_updated,
            url,
        };

        let names: Vec<String> = event_view
            .events
            .iter()
            .map(|e| match e.rules {
                Some(ref rule) => rule.clone(),
                None => "-".to_string(),
            })
            .collect();
        let mut event_select = SelectView::new();
        let index_width = ((names.len() + 1) as f64).log10() as usize + 1;
        for (i, label) in names.iter().enumerate() {
            let index_str = (i + 1).to_string();
            event_select.add_item(
                " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
                i,
            );
        }

        let event_view_tx_clone = event_view.event_view_tx.clone();
        event_select.set_on_submit(move |_s, i| {
            event_view_tx_clone
                .send(EventViewMessage::PrintEventProps(*i))
                .unwrap();
        });

        let quit_view = TextView::new("Press q to exit".to_string());
        let save_view = TextView::new("Press s to save changes.".to_string());
        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(event_select.scrollable().full_width().fixed_height(20))
            .child(DummyView)
            .child(DummyView)
            .child(save_view)
            .child(quit_view);

        let event_prop_box1 = BoxView::new(SizeConstraint::Full, SizeConstraint::Full, top_layout)
            .with_id("event_view");
        let event_prop_box2 = BoxView::new(
            SizeConstraint::Fixed(60),
            SizeConstraint::Fixed(60),
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("").with_id("event_properties"))
                    .child(
                        Dialog::around(TextView::new("Please Select an event from the left lists"))
                            .with_id("event_properties2"),
                    ),
            ),
        );

        event_view.cursive.add_layer(
            LinearLayout::horizontal()
                .child(event_prop_box1)
                .child(event_prop_box2)
                .scrollable(),
        );

        event_view.cursive.add_global_callback('q', |s| s.quit());
        let event_view_tx_clone = event_view.event_view_tx.clone();
        event_view.cursive.add_global_callback('s', move |_s| {
            event_view_tx_clone
                .send(EventViewMessage::SendUpdateRequest())
                .unwrap();
        });

        Ok(event_view)
    }

    pub fn run(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.event_view_rx.try_iter().next() {
                match message {
                    EventViewMessage::PrintEventProps(item) => {
                        let mut event_prop_window1 = self
                            .cursive
                            .find_id::<TextView>("event_properties")
                            .unwrap();
                        event_prop_window1.set_content(Event::get_event_properties(
                            &self.events[item],
                            &self.priority,
                            &self.qualifier,
                            &self.category,
                        ));

                        let mut event_prop_window2 =
                            self.cursive.find_id::<Dialog>("event_properties2").unwrap();
                        let event_view_tx_clone = self.event_view_tx.clone();

                        let mut qualifier_select = SelectView::new();
                        for (i, qualifier) in &self.qualifier {
                            qualifier_select.add_item(qualifier.to_string(), *i);
                        }
                        event_prop_window2.set_content(qualifier_select.on_submit(
                            move |_s, qualifier: &u32| {
                                event_view_tx_clone
                                    .send(EventViewMessage::SaveEventQualifier((item, *qualifier)))
                                    .unwrap();
                            },
                        ))
                    }

                    EventViewMessage::SaveEventQualifier(item) => {
                        if self.events[item.0].qualifier_id != item.1 {
                            self.events[item.0].qualifier_id = item.1;
                            self.is_event_updated
                                .insert(self.events[item.0].event_id, item.1);
                        }
                    }

                    EventViewMessage::SendUpdateRequest() => {
                        let mut resp_err: Vec<u32> = Vec::new();
                        let mut send_err: Vec<u32> = Vec::new();
                        for (event_id, qualifier_id) in &self.is_event_updated {
                            let url = format!(
                                "{}/api/event?event_id={}&qualifier_id={}",
                                self.url, event_id, qualifier_id
                            );
                            let client = reqwest::Client::new();
                            match client.put(url.as_str()).send() {
                                Ok(resp) => {
                                    if !resp.status().is_success() {
                                        resp_err.push(*event_id);
                                    }
                                }
                                Err(_) => send_err.push(*event_id),
                            }
                        }
                        if resp_err.is_empty() && send_err.is_empty() {
                            let popup_message = "Your changes have been successfully saved.";
                            EventView::create_popup_window(&mut self.cursive, popup_message, &"OK");
                        } else if !resp_err.is_empty() {
                            let popup_message = format!(
                                "Failed to update the following event_id:\n\n{:?}",
                                resp_err
                            );
                            EventView::create_popup_window(
                                &mut self.cursive,
                                popup_message.as_str(),
                                &"OK",
                            );
                        } else if !send_err.is_empty() {
                            let popup_message = format!("Failed to send update requests of the following event_id to backend server:\n\n{:?}", send_err);
                            EventView::create_popup_window(
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

            let size = events.len();

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
                        ((event.split_at(500)).0).to_string()
                    } else {
                        event.to_string()
                    }
                })
                .collect::<Vec<_>>();

            if examples.len() < 3 {
                loop {
                    if examples.len() == size || examples.len() == 3 {
                        break;
                    }
                    examples.push("<undecodable>".to_string());
                }
            }

            // REview displays at most 3 examples for each cluster
            if examples.len() >= 4 {
                let (examples, _) = examples.split_at(3);
                let cluster = Cluster {
                    cluster_id: *cluster_id,
                    signature,
                    size,
                    suspicious: "unknown".to_string(),
                    examples: examples.to_vec(),
                    event_ids,
                };
                clusters.push(cluster);
            } else {
                let cluster = Cluster {
                    cluster_id: *cluster_id,
                    signature,
                    size,
                    suspicious: "unknown".to_string(),
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
                        let mut cls = match ClusterView::read_clusters_file(self.cluster_path) {
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
                                // Cusive stops working without displaying
                                // above err_msg
                                HashMap::<usize, HashSet<u64>>::new()
                            }
                        };
                        if let Ok(mut raw_event_db) =
                            RawEventDatabase::new(&Path::new(self.raw_db_path))
                        {
                            let mut shrink_clusters: HashMap<usize, HashSet<u64>> = HashMap::new();
                            let mut event_ids_to_keep: Vec<u64> = Vec::new();
                            for (cluster_id, events) in cls.iter() {
                                let mut events_vec = events.iter().cloned().collect::<Vec<_>>();
                                if events.len() > 25 {
                                    use std::iter::FromIterator;
                                    events_vec.sort();
                                    let (_, events_vec) =
                                        events_vec.split_at(events_vec.len() - 25);
                                    shrink_clusters.insert(
                                        *cluster_id,
                                        HashSet::from_iter(events_vec.iter().map(|e| *e)),
                                    );
                                    event_ids_to_keep.extend_from_slice(&events_vec);
                                } else {
                                    event_ids_to_keep.extend_from_slice(&events_vec);
                                }
                            }
                            for (cluster_id, events) in shrink_clusters {
                                cls.insert(cluster_id, events);
                            }
                            event_ids_to_keep.sort();
                            match RawEventDatabase::shrink_to_fit(
                                &mut raw_event_db,
                                &event_ids_to_keep,
                            ) {
                                Ok(_) => {
                                    let mut file_path = match Path::new(self.cluster_path).parent()
                                    {
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
                                                // Cusive stops working without displaying
                                                // above err_msg
                                                String::new()
                                            }
                                        },
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
                                            // Cusive stops working without displaying
                                            // above err_msg
                                            String::new()
                                        }
                                    };
                                    if !file_path.ends_with('/') {
                                        file_path.push_str("/");
                                    }
                                    file_path
                                        .push_str(&Utc::now().format("%Y%m%d%H%M%S").to_string());
                                    file_path.push_str("_clusters");

                                    match ClusterView::write_clusters_file(
                                        &file_path.as_str(),
                                        &cls,
                                    ) {
                                        Ok(_) => {
                                            let msg = format!("Old events has been successfully deleted.\nNew cluster file {} is created.", file_path);
                                            ClusterView::create_popup_window_then_quit(
                                                &mut self.cursive,
                                                msg.as_str(),
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
                            }
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
                                .button("No", |s| s.quit())
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
                                    // Cusive stops working without displaying
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
                                // Cusive stops working without displaying
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
                .button("OK", |s| s.quit()),
        );
    }
}
