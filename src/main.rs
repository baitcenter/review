use clap::{App, Arg};

mod cluster;
mod event;

fn validate_url(url: &str) {
    if let Ok(url) = url::Url::parse(url) {
        match url.path_segments() {
            Some(mut path_segments) => {
                if path_segments.next() != Some("") {
                    eprintln!("Wrong url format. Please specify a url in the following format: http://<hostname>:<port number>");
                    std::process::exit(1);
                }
            }
            None => {
                eprintln!("Wrong url format. Please specify a url in the following format: http://<hostname>:<port number>");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Wrong url format. Please specify a url in the following format: http://<hostname>:<port number>");
        std::process::exit(1);
    }
}

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
                .conflicts_with("url") 
        )
        .arg(
            Arg::with_name("auto_labeling")
                .short("a")
                .long("auto_labeling")
                .takes_value(false)
                .help("Runs REview in auto labeling mode")
                .requires("dir_paths") 
                .conflicts_with("url") 
        )
        .arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .takes_value(true)
                .value_name("http://<hostname>:<port number>")
                .help("HTTP URL of backend server")
        )
        .get_matches();

    if let Some(url) = matches.value_of("url") {
        validate_url(url);
        let event_view = event::EventView::new(&url);
        match event_view {
            Ok(mut event_view) => event_view.run(),
            Err(e) => {
                eprintln!("Failed to create a event_view: {}", e);
                std::process::exit(1);
            }
        }
    } else {
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
}
