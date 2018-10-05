extern crate clap;

#[macro_use]
extern crate serde_derive;

mod cluster;

use clap::{App, Arg};
use cluster::read_clusters_from_file;

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
    match read_clusters_from_file(filename) {
        Ok(clusters) => println!("read {} clusters from {}.", clusters.len(), filename),
        Err(_) => println!("couldn't read the JSON file: {}", filename),
    }
}
