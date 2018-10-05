extern crate clap;

use clap::{App, Arg};

fn main() {
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        ).get_matches();
    println!("Loading JSON file: {}", matches.value_of("INPUT").unwrap());
}
