extern crate cursive;
extern crate serde;
extern crate serde_json;

use self::cursive::direction::Orientation;
use self::cursive::traits::*;
use self::cursive::view::{Position, SizeConstraint};
use self::cursive::views::{
    BoxView, Dialog, DummyView, EditView, LinearLayout, Panel, SelectView, TextView,
};
use self::cursive::Cursive;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::process;
use std::sync::mpsc;

#[derive(Serialize, Deserialize, Debug)]
pub struct Cluster {
    cluster_id: String,
    description: String,
    examples: Vec<String>,
    category: String,
    confidence: Option<f64>,
    qualifier: i32,
    signature: String,
    count: u32,
    status: Option<String>,
}

impl Cluster {
    pub fn get_cluster_properties(&self) -> String {
        format!("cluster_id: {}\ndescription: {}\nexamples: {:?}\ncategory: {}\nconfidence: {}\nqualifier: {}\nsignature: {}\ncount: {}\nstatus: {} \n\nPlease select the staus of this cluster\n\n",&self.cluster_id, &self.description, &self.examples, &self.category, &self.confidence.as_ref().unwrap_or(&(0 as f64)), &self.qualifier, &self.signature, &self.count, &self.status.as_ref().unwrap_or(&"-".to_string()))
    }

    pub fn read_clusters_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Cluster>, Box<Error>> {
        let json_file = File::open(path)?;
        let clusters: Vec<Cluster> = serde_json::from_reader(json_file)?;
        Ok(clusters)
    }

    pub fn write_clusters_to_file(clusters: &mut Vec<Cluster>, json_file: &String) {
        // Insert 0 for confidences and "-" for status if they are None.
        for i in 0..clusters.len() {
            if clusters[i].status.is_none() {
                clusters[i].status = Some("-".to_string());
            }
            if clusters[i].confidence.is_none() {
                clusters[i].confidence = Some(0 as f64);
            }
        }

        let file = File::create(json_file);
        let file = match file {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to write data to the file: {}", e);
                process::exit(1);
            }
        };

        match serde_json::to_writer(file, &clusters) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to write data to the file: {}", e);
                process::exit(1);
            }
        }
    }
}

pub struct ClusterView {
    cursive: Cursive,
    clview_rx: mpsc::Receiver<ClusterViewMessage>,
    clview_tx: mpsc::Sender<ClusterViewMessage>,
    clusters: Vec<Cluster>,
}

pub enum ClusterViewMessage {
    PrintClusterProps(usize),
    SaveClusterStatus((usize, String)),
    SaveClustersToJsonFile(String),
}

impl ClusterView {
    pub fn new(filename: &str) -> Result<ClusterView, String> {
        let (clview_tx, clview_rx) = mpsc::channel::<ClusterViewMessage>();

        let clusters: Vec<Cluster>;
        match Cluster::read_clusters_from_file(filename) {
            Ok(v) => {
                clusters = v;
            }
            Err(e) => {
                eprintln!("couldn't read the JSON file: {}", e);
                process::exit(1);
            }
        }

        let mut clview = ClusterView {
            cursive: Cursive::default(),
            clview_tx: clview_tx,
            clview_rx: clview_rx,
            clusters: clusters,
        };

        let names: Vec<String> = clview
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

        let clview_tx_clone = clview.clview_tx.clone();
        cluster_select.set_on_submit(move |_s, i| {
            clview_tx_clone
                .send(ClusterViewMessage::PrintClusterProps(*i))
                .unwrap();
        });

        let quit_view = TextView::new("Press q to exit".to_string());
        let save_view = TextView::new(
            "Please enter a file name (.json) and then press Enter key to save clustes to a file."
                .to_string(),
        );
        let clview_tx_clone = clview.clview_tx.clone();
        let save_filename = EditView::new()
            .on_submit(move |s, file: &str| {
                clview_tx_clone
                    .send(ClusterViewMessage::SaveClustersToJsonFile(file.to_string()))
                    .unwrap();
                s.screen_mut().add_layer_at(
                    Position::new(
                        cursive::view::Offset::Center,
                        cursive::view::Offset::Parent(5),
                    ),
                    Dialog::new()
                        .content(TextView::new("Saved"))
                        .dismiss_button("Back"),
                );
            }).fixed_width(15);

        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(cluster_select.scrollable().full_width().fixed_height(20))
            .child(DummyView)
            .child(DummyView)
            .child(save_view)
            .child(save_filename)
            .child(DummyView)
            .child(quit_view);

        let cl_prop_box1 = BoxView::new(SizeConstraint::Full, SizeConstraint::Full, top_layout)
            .with_id("cluster_view");

        let cl_prop_box2 = BoxView::new(
            SizeConstraint::Fixed(50),
            SizeConstraint::Fixed(50),
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("").with_id("cluster_properties"))
                    .child(
                        Dialog::around(TextView::new(
                            "Please Select the cluster name from the left lists",
                        )).with_id("cluster_properties2"),
                    ),
            ),
        );

        clview.cursive.add_layer(
            LinearLayout::horizontal()
                .child(cl_prop_box1)
                .child(cl_prop_box2),
        );

        clview.cursive.add_global_callback('q', |s| s.quit());

        Ok(clview)
    }

    pub fn run(&mut self) {
        while self.cursive.is_running() {
            while let Some(message) = self.clview_rx.try_iter().next() {
                match message {
                    ClusterViewMessage::PrintClusterProps(item) => {
                        let mut cl_prop_window1 = self
                            .cursive
                            .find_id::<TextView>("cluster_properties")
                            .unwrap();
                        cl_prop_window1
                            .set_content(Cluster::get_cluster_properties(&self.clusters[item]));

                        let mut cl_prop_window2 = self
                            .cursive
                            .find_id::<Dialog>("cluster_properties2")
                            .unwrap();
                        let clview_tx_clone = self.clview_tx.clone();
                        cl_prop_window2.set_content(
                            SelectView::new()
                                .item("suspicious", "suspicious")
                                .item("benign", "benign")
                                .item("unknown", "unknown")
                                .on_submit(move |_s, status: &str| {
                                    clview_tx_clone
                                        .send(ClusterViewMessage::SaveClusterStatus((
                                            item,
                                            status.to_string(),
                                        ))).unwrap();
                                }),
                        )
                    }

                    ClusterViewMessage::SaveClusterStatus(item) => {
                        self.clusters[Some(item.0).unwrap()].status = Some(item.1);
                    }

                    ClusterViewMessage::SaveClustersToJsonFile(filename) => {
                        Cluster::write_clusters_to_file(&mut self.clusters, &filename);
                    }
                }
            }

            self.cursive.step();
        }
    }
}
