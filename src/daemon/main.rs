use futures::prelude::*;
use hyper::service::service_fn;
use hyper::Server;

fn main() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set.");
    let addr = ([127, 0, 0, 1], 1337).into();

    let server = Server::bind(&addr)
        .serve(move || {
            let api_service = api_service::ApiService::new(&database_url)
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
}
