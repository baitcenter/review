use std::io;

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let server = match review::init() {
        Ok(server) => server,
        Err(e) => {
            log::error!("{:#}", e);
            std::process::exit(1);
        }
    };
    server.await
}
