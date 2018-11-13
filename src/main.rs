extern crate clap;
extern crate cursive;

#[macro_use]
extern crate serde_derive;

mod cluster;

use clap::{App, Arg};
use cluster::read_clusters_from_file;
use cluster::Cluster;
use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{Offset, Position};
use cursive::views::{Dialog, DummyView, LinearLayout, RadioGroup, SelectView, TextView};
use cursive::Cursive;

fn main() {
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        ).get_matches();
    let filename = matches.value_of("INPUT").unwrap();
    let clusters: Vec<Cluster>;
    match read_clusters_from_file(filename) {
        Ok(v) => {
            clusters = v;
        }
        Err(e) => {
            eprintln!("couldn't read the JSON file: {}", e);
            std::process::exit(1);
        }
    }

    let quit_view = TextView::new("Press q to quit".to_string())
        .with_id("quit")
        .full_width()
        .full_height();

    let names: Vec<String> = clusters.iter().map(|e| e.cluster_id.clone()).collect();
    let mut cluster_select = SelectView::new();
    let index_width = ((names.len() + 1) as f64).log10() as usize + 1;
    for (i, label) in names.iter().enumerate() {
        let index_str = (i + 1).to_string();
        cluster_select.add_item(
            " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
            i,
        );
    }

    cluster_select.set_on_submit(move |s, i| {
        let mut status_group: RadioGroup<String> = RadioGroup::new();
        s.screen_mut().add_layer_at(
            Position::new(Offset::Center, Offset::Parent(5)),
            Dialog::new()
                .content(
                    LinearLayout::vertical()
                        .child(TextView::new(Cluster::get_cluster_properties(
                            &clusters[*i],
                        ))).child(DummyView)
                        .child(DummyView)
                        .child(TextView::new("Please select the status of this cluster"))
                        .child(
                            LinearLayout::horizontal()
                                .child(status_group.button_str("Suspicious"))
                                .child(status_group.button_str("Benign"))
                                .child(status_group.button_str("Unknown")),
                        ).child(DummyView),
                ).button("Save", move |s| {
                    let selected_status = status_group.selection().to_string();

                    s.screen_mut().add_layer_at(
                        Position::new(Offset::Center, Offset::Parent(5)),
                        Dialog::new()
                            .content(LinearLayout::vertical().child(TextView::new(format!(
                                "You have selected {}",
                                selected_status
                            )))).dismiss_button("Ok"),
                    );
                }).dismiss_button("Back to the previous window"),
        );
    });

    let top_layout = LinearLayout::new(Orientation::Vertical)
        .child(cluster_select.scrollable().full_width().fixed_height(20))
        .child(DummyView)
        .child(quit_view);
    let mut siv = Cursive::default();
    siv.add_fullscreen_layer(top_layout);
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
}
