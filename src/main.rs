extern crate clap;

#[macro_use]
extern crate serde_derive;

mod cluster;
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

    let cluster_view = cluster::ClusterView::new(filename);
    match cluster_view {
        Ok(mut cluster_view) => cluster_view.run(),
        Err(e) => {
            eprintln!("Failed to create a cluster_view: {}", e);
            std::process::exit(1);
        }
    }
}

fn show_save_status_window(s: &mut Cursive) {
    s.screen_mut().add_layer_at(
        Position::new(Offset::Center, Offset::Parent(10)),
        Dialog::around(TextView::new("Would you like to save the status?"))
            .dismiss_button("YES")
            .dismiss_button("NO"),
    );
}
