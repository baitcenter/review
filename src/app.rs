use clap::{App, Arg};
use failure::ResultExt;

use crate::error::{Error, ErrorKind::Initialize, InitializeErrorReason};

fn create_app() -> App<'static, 'static> {
    App::new("REview")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Petabi, Inc.")
        .arg(Arg::with_name("subcommand")) // TODO: Remove in 0.8.0.
}

pub fn init() -> Result<(), Error> {
    let matches = create_app().get_matches();
    if matches.value_of("subcommand").is_some() {
        eprintln!("Warning: A subcommand is deprecated. Please run review without subcommand.");
    }
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context(Initialize(InitializeErrorReason::MissingDatabaseURL))?;
    let reviewd_addr = std::env::var("REVIEWD_ADDR")
        .context(Initialize(InitializeErrorReason::MissingReviewdAddr))?;
    let etcd_addr =
        std::env::var("ETCD_ADDR").context(Initialize(InitializeErrorReason::MissingEtcdAddr))?;
    let docker_host_ip = std::env::var("DOCKER_HOST_IP")
        .context(Initialize(InitializeErrorReason::MissingDockerHostIp))?;
    let kafka_url =
        std::env::var("KAFKA_URL").context(Initialize(InitializeErrorReason::MissingKafkaUrl))?;

    let docker_host_addr = format!("{}:8080", docker_host_ip);
    let etcd_url = format!("http://{}/v3beta/kv/put", etcd_addr);
    let reviewd_addr = reviewd_addr
        .parse::<std::net::SocketAddr>()
        .context(Initialize(InitializeErrorReason::REviewdUrl))?;

    let runner = super::Server::new(
        &database_url,
        &reviewd_addr,
        kafka_url,
        etcd_url,
        docker_host_addr,
    )
    .context(Initialize(InitializeErrorReason::BuildServer))?;

    runner
        .run()
        .context(Initialize(InitializeErrorReason::ServerRun))?;

    Ok(())
}
