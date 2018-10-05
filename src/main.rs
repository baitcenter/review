extern crate clap;

use clap::App;

fn main() {
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .get_matches();
}
