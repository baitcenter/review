extern crate clap;
extern crate cursive;

#[macro_use]
extern crate serde_derive;

mod cluster;

use clap::{App, Arg};
use cluster::read_clusters_from_file;
use cluster::Cluster;
use cursive::views::{Dialog, TextView};
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

    let mut siv = Cursive::default();
    siv.add_layer(
        Dialog::around(TextView::new(format!("Read {} clusters.", clusters.len())))
            .button("Quit", |s| s.quit()),
    );
    siv.run();
}
