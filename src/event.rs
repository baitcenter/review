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

        let category_url = format!("{}/category", url);
        let priority_url = format!("{}/priority", url);
        let qualifier_url = format!("{}/qualifier", url);
        let status_url = format!("{}/status", url);

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
        let event_url = format!("{}/event?status_id={}", url, review_id);
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
                                "{}/event?event_id={}&qualifier_id={}",
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
