use clap::{App, Arg};

mod cluster;

fn main() {
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(
            Arg::with_name("dir_paths")
                .short("d")
                .long("dir_paths")
                .takes_value(true)
                .value_name("dir01 dir02 dir03...")
                .min_values(1)
                .help("list of paths to directories containing cluster files in JSON format, separated by a space")
                .required(true),
        )
        .get_matches();

    let dir_paths: Vec<_> = matches.values_of("dir_paths").unwrap().collect();
    let cluster_view = cluster::ClusterView::new(&dir_paths);

    match cluster_view {
        Ok(mut cluster_view) => cluster_view.run(),
        Err(e) => {
            eprintln!("Failed to create a cluster_view: {}", e);
            std::process::exit(1);
        }
    }
}
