use clap::{App, Arg};
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

fn main() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set.");

    let matches = App::new("REviewd")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("reviewd configuration file")
                .required(true)
                .help("Path to a reviewd configuration file"),
        )
        .get_matches();

    let config = matches.value_of("config").unwrap();
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
