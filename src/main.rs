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
                .help("A space-separated list of paths to directories containing cluster files in JSON format")
                .required(true),
        )
        .arg(
            Arg::with_name("auto_labeling")
                .short("a")
                .long("auto_labeling")
                .takes_value(false)
                .help("Runs REview in auto labeling mode")
        )
        .get_matches();

    let dir_paths: Vec<_> = matches.values_of("dir_paths").unwrap().collect();

    if matches.is_present("auto_labeling") {
        cluster::ClusterView::run_auto_labeling_mode(&dir_paths);
    } else {
        let cluster_view = cluster::ClusterView::new(&dir_paths);
        match cluster_view {
            Ok(mut cluster_view) => cluster_view.run_feedback_mode(),
            Err(e) => {
                eprintln!("Failed to create a cluster_view: {}", e);
                std::process::exit(1);
            }
        }
    }
}
