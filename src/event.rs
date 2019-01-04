extern crate cursive;
extern crate quale;
extern crate sqlite;

use self::cursive::direction::Orientation;
use self::cursive::traits::*;
use self::cursive::view::{Position, SizeConstraint};
use self::cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use self::cursive::Cursive;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::process;
use std::sync::mpsc;

pub struct Event {
    event_id: i64,
    description: Option<String>,
    cluster_id: Option<String>,
    rules: Option<String>,
    category_id: i64,
    detector_id: i64,
    examples: Option<String>,
    priority_id: i64,
    qualifier_id: i64,
    status_id: i64,
    signature: String,
    is_updated: bool,
}

impl Event {
    pub fn get_event_properties(
        &self,
        priority: &HashMap<i64, String>,
        qualifier: &HashMap<i64, String>,
        category: &HashMap<i64, String>,
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

    pub fn read_table_from_database(
        database_name: &str,
        table_name: &str,
    ) -> Result<HashMap<i64, String>, Box<Error>> {
        let connection = sqlite::open(database_name)?;
        let sql_cmd = format!("select * from {};", table_name);
        let table_id = format!("{}_id", table_name);
        let table_value = table_name;

        let mut statement = connection.prepare(sql_cmd)?;
        let mut data: HashMap<i64, String> = HashMap::new();
        while let sqlite::State::Row = statement.next()? {
            let mut key: i64 = 0;
            let mut value: String = "".to_string();

            for i in 0..statement.count() {
                if statement.name(i).to_string().to_lowercase() == table_id.to_lowercase() {
                    key = statement.read::<i64>(i)?;
                } else if statement.name(i).to_string().to_lowercase()
                    == table_value.to_string().to_lowercase()
                {
                    value = statement.read::<String>(i)?;
                }
            }
            data.insert(key, value);
        }

        Ok(data)
    }

    pub fn read_events_from_database(
        database_filename: &str,
        status_review: i64,
    ) -> Result<Vec<Event>, Box<Error>> {
        let connection = sqlite::open(database_filename)?;
        let sql_cmd = format!("select * from events where status_id = {};", status_review);

        let mut events_statement = connection.prepare(sql_cmd)?;
        let mut events: Vec<Event> = Vec::new();
        let mut events_table: HashMap<usize, String> = HashMap::new();
        for i in 0..events_statement.count() {
            events_table.insert(i, events_statement.name(i).to_string());
        }

        while let sqlite::State::Row = events_statement.next().unwrap() {
            let mut event = Event {
                event_id: 0,
                cluster_id: None,
                rules: None,
                description: None,
                category_id: 0,
                detector_id: 0,
                examples: None,
                priority_id: 0,
                qualifier_id: 0,
                status_id: 0,
                signature: String::new().to_string(),
                is_updated: false,
            };

            for column_id in 0..events_statement.count() {
                if events_table.get(&column_id).unwrap().to_lowercase() == "event_id" {
                    event.event_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "cluster_id" {
                    match events_statement.read::<String>(column_id) {
                        Ok(value) => event.cluster_id = Some(value),
                        Err(_) => event.cluster_id = None,
                    }
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "rules" {
                    match events_statement.read::<String>(column_id) {
                        Ok(value) => event.rules = Some(value),
                        Err(_) => event.rules = None,
                    }
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "description" {
                    match events_statement.read::<String>(column_id) {
                        Ok(value) => event.description = Some(value),
                        Err(_) => event.description = None,
                    }
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "category_id" {
                    event.category_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "detector_id" {
                    event.detector_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "examples" {
                    match events_statement.read::<String>(column_id) {
                        Ok(value) => event.examples = Some(value),
                        Err(_) => event.examples = None,
                    }
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "priority_id" {
                    event.priority_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "qualifier_id" {
                    event.qualifier_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "status_id" {
                    event.status_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "signature" {
                    event.signature = events_statement.read::<String>(column_id)?;
                }
            }
            events.push(event);
        }

        Ok(events)
    }

    pub fn write_changes_to_database(
        cursive: &mut Cursive,
        database_filename: &str,
        events: &mut Vec<Event>,
        status: &HashMap<i64, String>,
    ) -> Result<usize, Box<Error>> {
        for event in events.iter().filter(|e| e.is_updated) {
            let connection = sqlite::open(database_filename)?;
            // For now, just set active for all the updated events.
            let sql_cmd = format!(
                "update events set status_id = {}, qualifier_id = {} where event_id = {};",
                &status
                    .iter()
                    .find(|&x| x.1.to_lowercase() == "active")
                    .unwrap()
                    .0,
                event.qualifier_id,
                event.event_id,
            );
            connection.execute(sql_cmd)?;
        }

        let popup_message = if events.iter().any(|&ref x| x.is_updated) {
            "Saved!".to_string()
        } else {
            "Nothing to save!".to_string()
        };
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .dismiss_button("Back"),
        );

        Ok(1)
    }
}

pub struct EventView {
    cursive: Cursive,
    event_view_rx: mpsc::Receiver<EventViewMessage>,
    event_view_tx: mpsc::Sender<EventViewMessage>,
    events: Vec<Event>,
    status: HashMap<i64, String>,
    priority: HashMap<i64, String>,
    qualifier: HashMap<i64, String>,
    category: HashMap<i64, String>,
    action: HashMap<i64, String>,
    central_dbname: String,
    rcvg_dbname: String,
}

pub enum EventViewMessage {
    PrintEventProps(usize),
    SaveEventQualifier((usize, i64)),
    SaveChangesToDatabase(),
    InvokeEventorProcess(),
}

impl EventView {
    pub fn new(central_dbname: &str, rcvg_dbname: &str) -> Result<EventView, Box<Error>> {
        let (event_view_tx, event_view_rx) = mpsc::channel::<EventViewMessage>();
        let status =
            Event::read_table_from_database(&central_dbname.to_string(), &"Status".to_string());
        if let Err(e) = status {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }
        let status = status.unwrap();

        let priority =
            Event::read_table_from_database(&central_dbname.to_string(), &"Priority".to_string());
        if let Err(e) = priority {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let qualifier =
            Event::read_table_from_database(&central_dbname.to_string(), &"Qualifier".to_string());
        if let Err(e) = qualifier {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let category =
            Event::read_table_from_database(&central_dbname.to_string(), &"Category".to_string());
        if let Err(e) = category {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let action =
            Event::read_table_from_database(&central_dbname.to_string(), &"Action".to_string());
        if let Err(e) = action {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let events = Event::read_events_from_database(
            &central_dbname.to_string(),
            *status
                .iter()
                .find(|&x| x.1.to_lowercase() == "review")
                .unwrap()
                .0,
        );
        if let Err(e) = events {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let mut event_view = EventView {
            cursive: Cursive::default(),
            event_view_tx,
            event_view_rx,
            events: events.unwrap(),
            status,
            priority: priority.unwrap(),
            qualifier: qualifier.unwrap(),
            category: category.unwrap(),
            action: action.unwrap(),
            central_dbname: central_dbname.to_string(),
            rcvg_dbname: rcvg_dbname.to_string(),
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
        let save_view = TextView::new("Press s to save changes to the database.".to_string());
        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(event_select.scrollable().full_width().fixed_height(20))
            .child(DummyView)
            .child(DummyView)
            .child(save_view)
            .child(quit_view);

        let event_prop_box1 = BoxView::new(SizeConstraint::Full, SizeConstraint::Full, top_layout)
            .with_id("event_view");
        let event_prop_box2 = BoxView::new(
            SizeConstraint::Fixed(50),
            SizeConstraint::Fixed(50),
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
                .child(event_prop_box2),
        );
        let event_view_tx_clone = event_view.event_view_tx.clone();
        event_view.cursive.add_global_callback('q', move |_s| {
            event_view_tx_clone
                .send(EventViewMessage::InvokeEventorProcess())
                .unwrap();
        });
        let event_view_tx_clone = event_view.event_view_tx.clone();
        event_view.cursive.add_global_callback('s', move |_s| {
            event_view_tx_clone
                .send(EventViewMessage::SaveChangesToDatabase())
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
                        for i in 1..=self.qualifier.len() {
                            let qualifier = self.qualifier.get(&(i as i64)).unwrap();
                            qualifier_select.add_item((*qualifier.clone()).to_string(), i as i64);
                        }
                        event_prop_window2.set_content(qualifier_select.on_submit(
                            move |_s, qualifier: &i64| {
                                event_view_tx_clone
                                    .send(EventViewMessage::SaveEventQualifier((item, *qualifier)))
                                    .unwrap();
                            },
                        ))
                    }

                    EventViewMessage::SaveEventQualifier(item) => {
                        if self.events[item.0].qualifier_id != item.1 {
                            self.events[item.0].qualifier_id = item.1;
                            self.events[item.0].is_updated = true;
                        }
                    }

                    EventViewMessage::SaveChangesToDatabase() => {
                        if let Err(e) = Event::write_changes_to_database(
                            &mut self.cursive,
                            &self.central_dbname,
                            &mut self.events,
                            &self.status,
                        ) {
                            let popup_message =
                                format!("Failed to save changes to database: {}", e);
                            EventView::create_popup_window(
                                &mut self.cursive,
                                &popup_message.as_str(),
                                &"Quit",
                            );
                        }
                    }

                    EventViewMessage::InvokeEventorProcess() => {
                        EventView::insert_records_into_ready_table(&self);

                        let is_updated = &self.events.iter().find(|&x| x.is_updated);
                        if is_updated.is_some() {
                            let eventor_path = quale::which("eventor")
                                .unwrap_or_else(|| Path::new("not found").to_path_buf());
                            if eventor_path.to_str().unwrap() == "not found" {
                                EventView::create_popup_window(
                                    &mut self.cursive,
                                    &"Unable to find the path to eventor.",
                                    &"Quit without invoking Eventor process",
                                );
                            } else {
                                let central_dbname_option =
                                    format!("--central_db {}", &self.central_dbname);
                                let rcvg_dbname_option = format!("--rcvg_db {}", &self.rcvg_dbname);

                                let output = process::Command::new(eventor_path.clone())
                                    .arg("publish")
                                    .arg(central_dbname_option.clone())
                                    .arg(rcvg_dbname_option.clone())
                                    .output();

                                match output {
                                    Ok(_) => {
                                        let popup_message = format!(
                                            "The following command is executed:\n\ncommand: {:?} {} {} \n",
                                            eventor_path, central_dbname_option, rcvg_dbname_option
                                        );
                                        EventView::create_popup_window(
                                            &mut self.cursive,
                                            &popup_message.as_str(),
                                            &"Quit",
                                        );
                                    }
                                    Err(e) => {
                                        let popup_message =
                                            format!("Failed to execute eventor: {}", e);
                                        EventView::create_popup_window(
                                            &mut self.cursive,
                                            &popup_message.as_str(),
                                            &"Quit",
                                        );
                                    }
                                };
                            }
                        } else {
                            EventView::create_popup_window(
                                &mut self.cursive,
                                &"Nothing is saved!".to_string(),
                                &"Quit without invoking Eventor process",
                            );
                        }
                    }
                }
            }

            self.cursive.step();
        }
    }

    pub fn insert_records_into_ready_table(&self) {
        for event in self.events.iter().filter(|e| e.is_updated) {
            let connection = sqlite::open(&self.central_dbname).unwrap();
            let action_id = *self
                .action
                .iter()
                .find(|&x| x.1.to_lowercase() == "update")
                .unwrap()
                .0;
            let sql_cmd = format!(
                "insert into ready_table (action_id, event_id, detector_id) Values({}, {}, {});",
                action_id, event.event_id, event.detector_id,
            );
            connection.execute(sql_cmd).unwrap();
        }
    }

    pub fn create_popup_window(cursive: &mut Cursive, popup_message: &str, button_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .dismiss_button(button_message),
        );
        cursive.quit();
    }
}
