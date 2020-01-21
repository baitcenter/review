use actix_files::Files;
use actix_web::{
    dev::Server,
    middleware,
    web::{Data, JsonConfig},
    App, HttpServer, Result,
};
use bigdecimal::FromPrimitive;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::io;
use thiserror::Error;

use crate::database::update_event_ids;
use crate::kafka_consumer;

mod route;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not bind server address: {0}")]
    Bind(io::Error),
    #[error("could not connect to database: {0}")]
    DatabaseConnection(r2d2::Error),
    #[error("could not initialize/migrate database: {0}")]
    DatabaseMigration(diesel_migrations::RunMigrationsError),
    #[error("could not create a database conenction pool: {0}")]
    PoolInitialization(r2d2::Error),
}

embed_migrations!();

#[allow(clippy::mutex_atomic)]
pub fn run(
    database_url: &str,
    reviewd_addr: &std::net::SocketAddr,
    kafka_url: String,
) -> Result<Server, Error> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::new(manager).map_err(Error::PoolInitialization)?;
    let conn = pool.get().map_err(Error::DatabaseConnection)?;
    embedded_migrations::run(&conn).map_err(Error::DatabaseMigration)?;

    let frontend_path = if let Ok(path) = std::env::var("FRONTEND_DIR") {
        path
    } else {
        eprintln!("Warning: FRONTEND_DIR is not set. Will use the current directory.");
        ".".to_string()
    };
    let config = kafka_consumer::KafkaConfig::new(
        kafka_url,
        reviewd_addr.to_string(),
        std::env::var("TASK_TIME_INTERVAL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok()),
        std::env::var("MAX_OFFSET_COUNT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok()),
    );
    if config.interval() != Some(0) {
        std::thread::spawn(move || {
            let _ = tokio::runtime::Runtime::new()
                .expect("Unable to create Tokio runtime")
                .block_on(kafka_consumer::KafkaConfig::periodically_fetch_kafka_message(config));
        });
    }
    let max_event_id_num = std::env::var("MAX_EVENT_ID_NUM")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| 25);
    log::info!(
        "The maximum number of event_ids per cluster/outlier is {}",
        max_event_id_num
    );
    if let (Ok(conn), Some(num)) = (pool.get(), FromPrimitive::from_usize(max_event_id_num)) {
        let _ = update_event_ids(&conn, num);
    }
    let max_event_id_num = Data::new(std::sync::Mutex::new(max_event_id_num));
    let server = HttpServer::new(move || {
        App::new()
            .data(JsonConfig::default().limit(1_048_576))
            .data(pool.clone())
            .app_data(max_event_id_num.clone())
            .configure(route::init_app)
            .service(Files::new("/", frontend_path.as_str()).index_file("index.html"))
            .wrap(middleware::Logger::default())
    })
    .bind(reviewd_addr)
    .map_err(Error::Bind)?
    .run();
    Ok(server)
}
