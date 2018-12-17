extern crate clap;
mod event;
use clap::{App, Arg};

fn main() {
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(
            Arg::with_name("central_db")
                .short("c")
                .long("central_db")
                .takes_value(true)
                .value_name("central_db")
                .help("Path to the central database file")
                .required(true),
        ).arg(
            Arg::with_name("rcvg_db")
                .short("r")
                .long("rcvg_db")
                .takes_value(true)
                .value_name("rcvg_db")
                .help("Path to the REconverge database file")
                .required(true),
        ).get_matches();
    let central_db = matches.value_of("central_db").unwrap();
    let rcvg_db = matches.value_of("rcvg_db").unwrap();
    let event_view = event::EventView::new(central_db, rcvg_db);
    match event_view {
        Ok(mut event_view) => event_view.run(),
        Err(e) => {
            eprintln!("Failed to create a event_view: {}", e);
            std::process::exit(1);
        }
    }
}
