use cursive::view::Position;
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use reviewd::QualifierUpdate;

use crate::error::Error;
use crate::views::{ClusterSelectView, MainView};

pub struct ClusterView {
    cursive: Cursive,
}

impl ClusterView {
    pub fn new(url: &str) -> Result<Self, Error> {
        let main_view = MainView::from_reviewd(url)?;
        let mut cluster_view = Self {
            cursive: Cursive::default(),
        };

        cluster_view.cursive.add_layer(main_view);
        cluster_view.cursive.add_global_callback('q', Cursive::quit);
        cluster_view
            .cursive
            .add_global_callback('s', send_update_request);

        Ok(cluster_view)
    }

    pub fn run(&mut self) {
        self.cursive.run();
    }
}

fn send_update_request(s: &mut Cursive) {
    let (qualifier_update, url): (Vec<QualifierUpdate>, String) = s
        .call_on_id("cluster_select", |v: &mut ClusterSelectView| {
            let qualifier_update = v
                .clusters
                .updated_clusters
                .iter()
                .map(|(cluster_id, i)| QualifierUpdate {
                    cluster_id: cluster_id.clone(),
                    data_source: v.clusters[*i].data_source.clone(),
                    qualifier: v.clusters[*i].qualifier.clone(),
                })
                .collect();
            (qualifier_update, v.clusters.url.clone())
        })
        .unwrap();

    let url = format!("{}/api/cluster/qualifier", url);
    let popup_message = if qualifier_update.is_empty() {
        "There are no updated clusters"
    } else {
        let body = serde_json::to_string(&qualifier_update);
        if let Ok(body) = body {
            let body = reqwest::Body::from(body);
            let client = reqwest::Client::new();
            match client.put(url.as_str()).body(body).send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        "Your changes have been successfully saved."
                    } else {
                        "Failed to update the qualifiers for some reason."
                    }
                }
                Err(_) => "Failed to send the update request to REviewd",
            }
        } else {
            "Something went wrong"
        }
    };
    create_popup_window(s, popup_message, "OK");
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
