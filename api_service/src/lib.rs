use futures::future;
use futures::future::Future;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{header, Body, Method, Request, Response, StatusCode};
use serde_json::json;
use std::collections::HashMap;

mod error;
use error::Error;

#[derive(Clone)]
pub struct ApiService {
    db: db::DB,
    reviewd_addr: String,
    etcd_key: String,
    etcd_url: String,
    reviewd_url: String,
}

impl ApiService {
    pub fn new(
        database_url: &str,
        reviewd_addr: &str,
        etcd_url: &str,
        etcd_key: &str,
    ) -> Box<Future<Item = Self, Error = Error> + Send + 'static> {
        let reviewd_addr = reviewd_addr.to_string();
        let etcd_key = etcd_key.to_string();
        let etcd_url = etcd_url.to_string();
        let reviewd_url = database_url.to_string();

        let fut = db::DB::new(database_url)
            .and_then(move |db| {
                future::ok(Self {
                    db,
                    reviewd_addr,
                    etcd_key,
                    etcd_url,
                    reviewd_url,
                })
            })
            .map_err(Into::into);

        Box::new(fut)
    }

    pub fn request_handler(
        &self,
        req: Request<Body>,
    ) -> Box<Future<Item = Response<Body>, Error = Error> + Send + 'static> {
        match req.uri().query() {
            Some(query) => match (req.method(), req.uri().path()) {
                (&Method::GET, "/api/event") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if let Some(status_id) = hash_query.get("status_id") {
                        if let Ok(status_id) = status_id.parse::<i32>() {
                            let result = db::DB::get_event_by_status(&self.db, status_id)
                                .and_then(|data| match serde_json::to_string(&data) {
                                    Ok(json) => future::ok(
                                        Response::builder()
                                            .header(header::CONTENT_TYPE, "application/json")
                                            .body(Body::from(json))
                                            .unwrap(),
                                    ),
                                    Err(_) => future::ok(ApiService::build_http_500_response()),
                                })
                                .map_err(Into::into);

                            Box::new(result)
                        } else {
                            Box::new(future::ok(ApiService::build_http_404_response()))
                        }
                    } else if let Some(qualifier_id) = hash_query.get("qualifier_id") {
                        if let Ok(qualifier_id) = qualifier_id.parse::<i32>() {
                            let result = db::DB::get_signature_by_qualifier(&self.db, qualifier_id)
                                .and_then(|data| match serde_json::to_string(&data) {
                                    Ok(json) => future::ok(
                                        Response::builder()
                                            .header(header::CONTENT_TYPE, "application/json")
                                            .body(Body::from(json))
                                            .unwrap(),
                                    ),
                                    Err(_) => future::ok(ApiService::build_http_500_response()),
                                })
                                .map_err(Into::into);

                            Box::new(result)
                        } else {
                            Box::new(future::ok(ApiService::build_http_404_response()))
                        }
                    } else {
                        Box::new(future::ok(ApiService::build_http_404_response()))
                    }
                }
                (&Method::PUT, "/api/event") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if let (Some(event_id), Some(qualifier_id)) =
                        (hash_query.get("event_id"), hash_query.get("qualifier_id"))
                    {
                        if let (Ok(event_id), Ok(qualifier_id)) =
                            (event_id.parse::<i32>(), qualifier_id.parse::<i32>())
                        {
                            let benign_id = db::DB::get_benign_id(&self.db);
                            if qualifier_id == benign_id {
                                let value = format!(
                                    "http://{}/api/event?qualifier_id={}",
                                    &self.reviewd_addr, benign_id,
                                );
                                let data = format!(
                                    "{{\"key\": \"{}\", \"value\": \"{}\"}}",
                                    base64::encode(&self.etcd_key),
                                    base64::encode(&value)
                                );
                                let client = reqwest::Client::new();
                                if let Err(e) = client.post(&self.etcd_url).body(data).send() {
                                    eprintln!("An error occurs while updating etcd: {}", e);
                                }
                            } else if benign_id == -1 {
                                eprintln!("An error occurs while accessing database.");
                            }
                            let result =
                                db::DB::update_qualifier_id(&self.db, event_id, qualifier_id)
                                    .and_then(|_| {
                                        future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from("Database has been updated"))
                                                .unwrap(),
                                        )
                                    })
                                    .map_err(Into::into);

                            Box::new(result)
                        } else {
                            Box::new(future::ok(ApiService::build_http_404_response()))
                        }
                    } else {
                        Box::new(future::ok(ApiService::build_http_404_response()))
                    }
                }
                _ => Box::new(future::ok(ApiService::build_http_404_response())),
            },
            None => match (req.method(), req.uri().path()) {
                (&Method::GET, "/api/action") => {
                    let result = db::DB::get_action_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/category") => {
                    let result = db::DB::get_category_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/event") => {
                    let result = db::DB::get_event_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/priority") => {
                    let result = db::DB::get_priority_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/qualifier") => {
                    let result = db::DB::get_qualifier_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/status") => {
                    let result = db::DB::get_status_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .unwrap(),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }
                _ => Box::new(future::ok(ApiService::build_http_404_response())),
            },
        }
    }

    pub fn error_handler(
        res_body: Result<Response<Body>, Error>,
    ) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send + 'static> {
        match res_body {
            Ok(res) => Box::new(future::ok(res)),
            Err(err) => {
                let message = err.to_string();
                let status_code = StatusCode::INTERNAL_SERVER_ERROR;
                let body = json!({
                    "message": message,
                })
                .to_string();

                let res: Response<Body> = Response::builder()
                    .status(status_code)
                    .header(CONTENT_TYPE, "application/json")
                    .header(CONTENT_LENGTH, body.len().to_string().as_str())
                    .body(body.into())
                    .unwrap();

                Box::new(future::ok(res.map(Into::into)))
            }
        }
    }

    fn build_http_500_response() -> Response<Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Internal Server Error"))
            .unwrap()
    }

    fn build_http_404_response() -> Response<Body> {
        let body = Body::from("Not Found");
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body)
            .unwrap()
    }
}
