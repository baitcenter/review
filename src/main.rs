extern crate clap;
mod event;
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
    let filename = matches.value_of("INPUT").unwrap();

    let event_view = event::EventView::new(filename);
    match event_view {
        Ok(mut event_view) => event_view.run(),
        Err(e) => {
            eprintln!("Failed to create a event_view: {}", e);
            std::process::exit(1);
        }
    }
}
