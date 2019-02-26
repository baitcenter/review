use clap::{App, AppSettings, Arg, ArgGroup, SubCommand};
use futures::prelude::*;
use hyper::service::service_fn;
use hyper::Server;
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Config {
    reviewd_addr: String,
    etcd_addr: String,
    etcd_key: String,
}

fn read_config_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<std::error::Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;

    Ok(config)
}

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
    //   command hierarchy
    //
    //            review                      TOP
    //               |
    //  ----------------------------
    //   |                        |
    // reviewd                   cli          LEVEL 1
    //   |                     /     \
    // config             dir_paths  url      LEVEL 2
    //                        |
    //             (option: auto_labeling)    LEVEL 3
    //
    let matches = App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("cli")
            .about("Runs REview client modes")
            .arg(
                Arg::with_name("dir_paths")
                    .short("d")
                    .long("dir_paths")
                    .takes_value(true)
                    .value_name("dir01 dir02 dir03...")
                    .min_values(1)
                    .help("A space-separated list of paths to directories containing cluster files in JSON format")
            )
            .arg(
                Arg::with_name("url")
                    .short("u")
                    .long("url")
                    .takes_value(true)
                    .value_name("http://<hostname>:<port number>")
                    .help("HTTP URL of backend server")
            )
            .arg(
                Arg::with_name("auto_labeling")
                    .short("a")
                    .long("auto_labeling")
                    .takes_value(false)
                    .requires("dir_paths") 
                    .conflicts_with("url") 
                    .help("Runs REview in auto labeling mode")
            )
            .group(ArgGroup::with_name("review_option")
                .args(&["dir_paths", "url"])
                .required(true))
        )
        .subcommand(SubCommand::with_name("reviewd")
            .about("Runs REviewd (http server mode)")
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .takes_value(true)
                    .value_name("path to config file")
                    .help("reviewd configration file")
                    .required(true)
            )
        ).get_matches();

    if let Some(review_matches) = matches.subcommand_matches("cli") {
        if let Some(url) = review_matches.value_of("url") {
            validate_url(url);
            let event_view = cli::EventView::new(&url);
            match event_view {
                Ok(mut event_view) => event_view.run(),
                Err(e) => {
                    eprintln!("Failed to create a event_view: {}", e);
                    std::process::exit(1);
                }
            }
        } else if let Some(dir_paths) = review_matches.values_of("dir_paths") {
            let dir_paths: Vec<_> = dir_paths.collect();
            if matches.is_present("auto_labeling") {
                cli::ClusterView::run_auto_labeling_mode(&dir_paths);
            } else {
                let cluster_view = cli::ClusterView::new(&dir_paths);
                match cluster_view {
                    Ok(mut cluster_view) => cluster_view.run_feedback_mode(),
                    Err(e) => {
                        eprintln!("Failed to create a cluster_view: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    } else if let Some(reviewd_matches) = matches.subcommand_matches("reviewd") {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set.");
        let config = reviewd_matches.value_of("config").unwrap();
        match read_config_file(config) {
            Ok(config) => {
                if let Ok(reviewd_addr) = config.reviewd_addr.parse() {
                    let server = Server::bind(&reviewd_addr)
                        .serve(move || {
                            let etcd_url = format!("http://{}/v3/kv/put", config.etcd_addr);
                            let api_service = api_service::ApiService::new(
                                &database_url,
                                config.reviewd_addr.as_str(),
                                etcd_url.as_str(),
                                config.etcd_key.as_str(),
                            )
                            .map_err(|e| panic!("Initialization fails: {}", e))
                            .and_then(|srv| {
                                service_fn(move |req| {
                                    api_service::ApiService::request_handler(&srv.clone(), req)
                                        .then(api_service::ApiService::error_handler)
                                })
                            });
                            Box::new(api_service)
                        })
                        .map_err(|e| panic!("Failed to build server: {}", e));

                    hyper::rt::run(server);
                } else {
                    eprintln!("IP address and/or port number for reviewd is bad/illegal format.");
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse reviewd configuration file: {}", e);
                std::process::exit(1);
            }
        }
    }
}
