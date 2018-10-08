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
use cursive::views::{LinearLayout, SelectView, TextView, ViewRef};
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

    let examples = if clusters.len() > 0 {
        join_examples(&clusters[0].examples)
    } else {
        "".to_string()
    };
    let examples_view = TextView::new(examples)
        .with_id("examples")
        .full_width()
        .full_height();

    let names: Vec<String> = clusters.iter().map(|e| e.cluster_id.clone()).collect();
    let mut cluster_select = SelectView::new();
    let index_width = if names.len() == 0 {
        1
    } else {
        ((names.len() + 1) as f64).log10() as usize + 1
    };
    for (i, label) in names.iter().enumerate() {
        let index_str = (i + 1).to_string();
        cluster_select.add_item(
            " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
            i,
        );
    }
    cluster_select.set_on_select(move |s, i| {
        let mut examples_view: ViewRef<TextView> = s.find_id("examples").unwrap();
        examples_view.set_content(join_examples(&clusters[*i].examples));
    });

    let separator = TextView::new("").fixed_height(1);

    let top_layout = LinearLayout::new(Orientation::Vertical)
        .child(cluster_select.scrollable().full_width().fixed_height(6))
        .child(separator)
        .child(examples_view);
    let mut siv = Cursive::default();
    siv.add_fullscreen_layer(top_layout);
    siv.run();
}

fn join_examples(examples: &Vec<String>) -> String {
    if examples.len() > 0 {
        examples.join("\n\n")
    } else {
        "(no example)".to_string()
    }
}
