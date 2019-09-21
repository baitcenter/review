use clap::{App, AppSettings, Arg, ArgGroup, SubCommand};
use failure::ResultExt;
use futures::future;
use futures::prelude::*;
use hyper::service::service_fn;
use hyper::Server;

use crate::error::{Error, ErrorKind, InitializeErrorReason};

fn create_app() -> App<'static, 'static> {
    App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("client")
                .about("Runs REview client modes")
                .arg(
                    Arg::with_name("url")
                        .short("u")
                        .long("url")
                        .takes_value(true)
                        .value_name("http://<hostname>:<port number>")
                        .help("HTTP URL of backend server"),
                )
                .group(
                    ArgGroup::with_name("review_option")
                        .args(&["url"])
                        .required(true),
                ),
        )
        .subcommand(SubCommand::with_name("reviewd").about("Runs REviewd (http server mode)"))
}

pub fn init() -> Result<(), Error> {
    let matches = create_app().get_matches();
    if let Some(review_matches) = matches.subcommand_matches("client") {
        if let Some(url) = review_matches.value_of("url") {
            let parsed_url = url::Url::parse(url)
                .context(ErrorKind::Initialize(InitializeErrorReason::ClientUrl))?;
            if parsed_url.scheme() != "http"
                || parsed_url.path() != "/"
                || parsed_url.port().is_none()
            {
                return Err(Error::from(ErrorKind::Initialize(
                    InitializeErrorReason::ClientUrl,
                )));
            }
            let mut cluster_view = client::http::ClusterView::new(&url)
                .context(ErrorKind::Initialize(InitializeErrorReason::ClientMode))?;
            cluster_view.run();
        }
    } else if matches.subcommand_matches("reviewd").is_some() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").context(ErrorKind::Initialize(
            InitializeErrorReason::MissingDatabaseURL,
        ))?;
        let reviewd_addr = std::env::var("REVIEWD_ADDR").context(ErrorKind::Initialize(
            InitializeErrorReason::MissingReviewdAddr,
        ))?;
        let etcd_addr = std::env::var("ETCD_ADDR").context(ErrorKind::Initialize(
            InitializeErrorReason::MissingEtcdAddr,
        ))?;
        let docker_host_ip = std::env::var("DOCKER_HOST_IP").context(ErrorKind::Initialize(
            InitializeErrorReason::MissingDockerHostIp,
        ))?;
        let kafka_url = std::env::var("KAFKA_URL").context(ErrorKind::Initialize(
            InitializeErrorReason::MissingKafkaUrl,
        ))?;

        let docker_host_addr = format!("{}:8080", docker_host_ip);
        let etcd_url = format!("http://{}/v3beta/kv/put", etcd_addr);
        let reviewd_addr = reviewd_addr
            .parse::<std::net::SocketAddr>()
            .context(ErrorKind::Initialize(InitializeErrorReason::REviewdUrl))?;

        hyper::rt::run(future::lazy(move || {
            api_service::ApiService::new(&database_url, &docker_host_addr, &etcd_url, &kafka_url)
                .map_err(|e| panic!("Reviewd initialization fails: {}", e))
                .and_then(move |state| {
                    let service = move || {
                        let state = state.clone();
                        service_fn(move |req| {
                            api_service::ApiService::request_handler(state.clone(), req)
                                .then(api_service::ApiService::api_error_handler)
                        })
                    };
                    Server::bind(&reviewd_addr)
                        .serve(service)
                        .map_err(|e| panic!("Failed to build server: {}", e))
                })
        }));
    }

    Ok(())
}
