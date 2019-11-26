use actix_files::Files;
use actix_rt::SystemRunner;
use actix_web::{middleware, App, HttpServer, Result};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::io;
use thiserror::Error;

mod route;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not bind server address")]
    Bind(io::Error),
    #[error("could not connect to database")]
    DatabaseConnection(r2d2::Error),
    #[error("could not initialize/migrate database")]
    DatabaseMigration(diesel_migrations::RunMigrationsError),
    #[error("could not create a database conenction pool")]
    PoolInitialization(r2d2::Error),
}

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
        let runner = actix_rt::System::new("REview");

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::new(manager).map_err(|e| Error::PoolInitialization(e))?;
        let conn = pool.get().map_err(|e| Error::DatabaseConnection(e))?;
        embedded_migrations::run(&conn).map_err(|e| Error::DatabaseMigration(e))?;
        let etcd_server = EtcdServer {
            etcd_url,
            docker_host_addr,
        };

        let frontend_path = if let Ok(path) = std::env::var("FRONTEND_DIR") {
            path
        } else {
            eprintln!("Warning: FRONTEND_DIR is not set. Will use the current directory.");
            ".".to_string()
        };
        HttpServer::new(move || {
            App::new()
                .data(pool.clone())
                .data(kafka_url.clone())
                .data(etcd_server.clone())
                .configure(route::init_app)
                .service(Files::new("/", frontend_path.as_str()).index_file("index.html"))
                .wrap(middleware::Logger::default())
        })
        .bind(reviewd_addr)
        .map_err(|e| Error::Bind(e))?
        .start();

        Ok(Self { runner })
    }

    pub fn run(self) -> Result<(), std::io::Error> {
        self.runner.run()
    }
}
