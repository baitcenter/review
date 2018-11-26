extern crate cursive;
extern crate sqlite;

use self::cursive::direction::Orientation;
use self::cursive::traits::*;
use self::cursive::view::{Position, SizeConstraint};
use self::cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use self::cursive::Cursive;
use std::collections::HashMap;
use std::error::Error;
use std::process;
use std::sync::mpsc;

pub struct Event {
    event_id: i64,
    description: Option<String>,
    cluster_id: i64,
    rules: Option<String>,
    category_id: i64,
    detector_id: i64,
    examples: Option<String>,
    priority_id: i64,
    qualifier_id: i64,
    status_id: i64,
    is_updated: bool,
}

impl Event {
    pub fn get_event_properties(
        &self,
        status: &HashMap<i64, String>,
        priority: &HashMap<i64, String>,
        qualifier: &HashMap<i64, String>,
        category: &HashMap<i64, String>,
    ) -> String {
        format!(
            "event_id: {}\ncluster_id: {}\nrules: {}\ndescription: {}\ncategory_id: {}\ndetector_id: {}\nexamples: {}\npriority_id: {}\nqualifier_id: {}\nstatus_id: {}\n\n",
            &self.event_id,
            &self.cluster_id,
            &self.rules.as_ref().unwrap_or(&"-".to_string()),
            &self.description.as_ref().unwrap_or(&"-".to_string()),
            category.get(&self.category_id).unwrap(),
            &self.detector_id,
            &self.examples.as_ref().unwrap_or(&"-".to_string()),
            priority.get(&self.priority_id).unwrap(),
            qualifier.get(&self.qualifier_id).unwrap(),
            status.get(&self.status_id).unwrap(),
        )
    }

    pub fn read_table_from_database(
        database_name: &String,
        table_name: &String,
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
        database_filename: &String,
        status_review: &i64,
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
                cluster_id: 0,
                rules: None,
                description: None,
                category_id: 0,
                detector_id: 0,
                examples: None,
                priority_id: 0,
                qualifier_id: 0,
                status_id: 0,
                is_updated: false,
            };

            for column_id in 0..events_statement.count() {
                if events_table.get(&column_id).unwrap().to_lowercase() == "event_id" {
                    event.event_id = events_statement.read::<i64>(column_id)?;
                } else if events_table.get(&column_id).unwrap().to_lowercase() == "cluster_id" {
                    event.cluster_id = events_statement.read::<i64>(column_id)?;
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
                } else {
                    eprintln!(
                        "Error! Unexpected database schema found: {}",
                        events_table.get(&column_id).unwrap(),
                    );
                    process::exit(1);
                }
            }
            events.push(event);
        }

        Ok(events)
    }

    pub fn write_changes_to_database(
        cursive: &mut Cursive,
        database_filename: &String,
        events: &mut Vec<Event>,
        status: &HashMap<i64, String>,
        action: &HashMap<i64, String>,
    ) -> Result<usize, Box<Error>> {
        let mut event_keys: Vec<usize> = Vec::new();
        for i in 0..events.len() {
            if events[i].is_updated == true {
                event_keys.push(i);
            }
        }

        let mut popup_message = "Nothing to save!".to_string();
        if !event_keys.is_empty() {
            for key in 0..event_keys.len() {
                let connection = sqlite::open(database_filename)?;
                // For now, just set active for all the updated events.
                let mut sql_cmd = format!(
                    "update events set status_id = {}, qualifier_id = {} where event_id = {};",
                    &status
                        .iter()
                        .find(|&x| x.1.to_lowercase() == "active")
                        .unwrap()
                        .0,
                    events[event_keys[key]].qualifier_id,
                    events[event_keys[key]].event_id,
                );

                connection.execute(sql_cmd)?;
                events[event_keys[key]].status_id = *status
                    .iter()
                    .find(|&x| x.1.to_lowercase() == "active")
                    .unwrap()
                    .0;

                sql_cmd = format!(
                    "select * from ready_table where event_id = {};",
                    events[event_keys[key]].event_id
                );
                let mut ready_table_cursor = connection.prepare(sql_cmd).unwrap().cursor();
                match ready_table_cursor.next().unwrap() {
                    Some(_) => {
                        sql_cmd = format!(
                            "update ready_table set action_id = {} where event_id = {}",
                            *action
                                .iter()
                                .find(|&x| x.1.to_lowercase() == "update")
                                .unwrap()
                                .0,
                            events[event_keys[key]].event_id,
                        );
                        connection.execute(sql_cmd)?;
                    }
                    None => {
                        sql_cmd = "select * from ready_table;".to_string();
                        let statement = connection.prepare(sql_cmd).unwrap();
                        let columns = statement.names();

                        sql_cmd = "select max(publish_id) from ready_table;".to_string();
                        let mut cursor = connection.prepare(sql_cmd).unwrap().cursor();

                        let publish_id =
                            cursor.next().unwrap().unwrap()[0].as_integer().unwrap() + 1;
                        let action_id = *action
                            .iter()
                            .find(|&x| x.1.to_lowercase() == "update")
                            .unwrap()
                            .0;
                        let event_id = events[event_keys[key]].event_id;
                        let detector_id = events[event_keys[key]].detector_id;

                        let mut values: Vec<i64> = Vec::new();
                        for c in 0..columns.len() {
                            if columns[c].to_lowercase() == "publish_id" {
                                values.push(publish_id);
                            } else if columns[c].to_lowercase() == "action_id" {
                                values.push(action_id);
                            } else if columns[c].to_lowercase() == "event_id" {
                                values.push(event_id);
                            } else if columns[c].to_lowercase() == "detector_id" {
                                values.push(detector_id);
                            } else if columns[c].to_lowercase() == "time_published" {
                                values.push(0);
                            } else {
                                eprintln!(
                                    "Error! Unexpected database schema found: {}",
                                    columns[c],
                                );
                                process::exit(1);
                            }
                        }

                        sql_cmd = "insert into ready_table Values(".to_string();
                        for key in 0..values.len() {
                            if key == 0 {
                                sql_cmd = format!("{}{}", sql_cmd, values[key]);
                            } else {
                                sql_cmd = format!("{}, {}", sql_cmd, values[key]);
                            }
                        }
                        sql_cmd = format!("{});", sql_cmd);
                        connection.execute(sql_cmd)?;
                    }
                }
            }
            popup_message = "Saved!".to_string();
        }

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
    database_filename: String,
}

pub enum EventViewMessage {
    PrintEventProps(usize),
    SaveEventQualifier((usize, i64)),
    SaveChangesToDatabase(),
}

impl EventView {
    pub fn new(database_name: &str) -> Result<EventView, Box<Error>> {
        let (event_view_tx, event_view_rx) = mpsc::channel::<EventViewMessage>();
        let status =
            Event::read_table_from_database(&database_name.to_string(), &"Status".to_string());
        if let Err(e) = status {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }
        let status = status.unwrap();

        let priority =
            Event::read_table_from_database(&database_name.to_string(), &"Priority".to_string());
        if let Err(e) = priority {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let qualifier =
            Event::read_table_from_database(&database_name.to_string(), &"Qualifier".to_string());
        if let Err(e) = qualifier {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let category =
            Event::read_table_from_database(&database_name.to_string(), &"Category".to_string());
        if let Err(e) = category {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let action =
            Event::read_table_from_database(&database_name.to_string(), &"Action".to_string());
        if let Err(e) = action {
            eprintln!("Failed to read database: {}", e);
            process::exit(1);
        }

        let events = Event::read_events_from_database(
            &database_name.to_string(),
            &status
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
            event_view_tx: event_view_tx,
            event_view_rx: event_view_rx,
            events: events.unwrap(),
            status: status,
            priority: priority.unwrap(),
            qualifier: qualifier.unwrap(),
            category: category.unwrap(),
            action: action.unwrap(),
            database_filename: database_name.to_string(),
        };

        let names: Vec<String> = event_view
            .events
            .iter()
            .map(|e| match e.rules {
                Some(ref rule) => rule.clone(),
                None => "-".to_string(),
            }).collect();
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

        event_view.cursive.add_global_callback('q', |s| s.quit());
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
                            &self.status,
                            &self.priority,
                            &self.qualifier,
                            &self.category,
                        ));

                        let mut event_prop_window2 =
                            self.cursive.find_id::<Dialog>("event_properties2").unwrap();
                        let event_view_tx_clone = self.event_view_tx.clone();

                        let mut qualifier_select = SelectView::new();
                        for i in 1..self.qualifier.len() + 1 {
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
                            &self.database_filename,
                            &mut self.events,
                            &self.status,
                            &self.action,
                        ) {
                            eprintln!("Failed to write results to database: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }

            self.cursive.step();
        }
    }
}
