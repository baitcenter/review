use actix::prelude::SystemRunner;
use actix_web::{middleware, App, HttpServer};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::ResultExt;

use self::error::{Error, ErrorKind::Initialize, InitializeErrorReason};

mod error;
mod route;

embed_migrations!();

#[derive(Clone, Debug)]
pub(crate) struct EtcdServer {
    pub(crate) etcd_url: String,
    pub(crate) docker_host_addr: String,
}

pub struct Server {
    runner: SystemRunner,
}

impl Server {
    pub fn new(
        database_url: &str,
        reviewd_addr: &std::net::SocketAddr,
        kafka_url: String,
        etcd_url: String,
        docker_host_addr: String,
    ) -> Result<Self, Error> {
        let runner = actix::System::new("REviewd");

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool =
            Pool::new(manager).context(Initialize(InitializeErrorReason::PoolInitialization))?;
        let conn = pool
            .get()
            .context(Initialize(InitializeErrorReason::DatabaseConnection))?;
        embedded_migrations::run(&conn)
            .context(Initialize(InitializeErrorReason::DatabaseSchema))?;
        let etcd_server = EtcdServer {
            etcd_url,
            docker_host_addr,
        };

        HttpServer::new(move || {
            App::new()
                .data(pool.clone())
                .data(kafka_url.clone())
                .data(etcd_server.clone())
                .configure(route::init_app)
                .wrap(middleware::Logger::default())
        })
        .bind(reviewd_addr)
        .context(Initialize(InitializeErrorReason::Bind))?
        .start();

        Ok(Self { runner })
    }

    pub fn run(self) -> Result<(), std::io::Error> {
        self.runner.run()
    }
}
