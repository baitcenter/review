use futures::future;
use futures::future::Future;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::rt::Stream;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

mod error;
use error::Error;

#[derive(Clone)]
pub struct ApiService {
    db: db::DB,
    docker_host_addr: String,
    etcd_key: String,
    etcd_url: String,
    reviewd_url: String,
}

impl ApiService {
    pub fn new(
        database_url: &str,
        docker_host_addr: &str,
        etcd_url: &str,
        etcd_key: &str,
    ) -> Box<Future<Item = Self, Error = Error> + Send + 'static> {
        let docker_host_addr = docker_host_addr.to_string();
        let etcd_key = etcd_key.to_string();
        let etcd_url = etcd_url.to_string();
        let reviewd_url = database_url.to_string();

        let fut = db::DB::new(database_url)
            .and_then(move |db| {
                future::ok(Self {
                    db,
                    docker_host_addr,
                    etcd_key,
                    etcd_url,
                    reviewd_url,
                })
            })
            .map_err(Into::into);

        Box::new(fut)
    }

    pub fn request_handler(
        self,
        req: Request<Body>,
    ) -> Box<Future<Item = Response<Body>, Error = Error> + Send + 'static> {
        match req.uri().query() {
            Some(query) => match (req.method(), req.uri().path()) {
                (&Method::GET, "/api/cluster") => {
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
                    } else if let (Some(cluster_id), Some(max_cluster_count)) = (
                        hash_query.get("cluster_id"),
                        hash_query.get("max_cluster_count"),
                    ) {
                        #[derive(Debug, Serialize)]
                        struct Clusters {
                            cluster_id: Option<String>,
                            examples: Option<Vec<(usize, String)>>,
                        }
                        if cluster_id == "all" && max_cluster_count == "all" {
                            Box::new(future::ok(
                                Response::builder()
                                    .status(StatusCode::BAD_REQUEST)
                                    .body(Body::from("Invalid request"))
                                    .unwrap(),
                            ))
                        } else if cluster_id == "all" {
                            if let Ok(max_cluster_count) = max_cluster_count.parse::<usize>() {
                                if max_cluster_count == 0 {
                                    return Box::new(future::ok(
                                        Response::builder()
                                            .status(StatusCode::BAD_REQUEST)
                                            .body(Body::from("max_cluster_count must be a positive integer value or 'all'"))
                                            .unwrap(),
                                    ));
                                }

                                let result = db::DB::get_all_clusters_with_limit_num(
                                    &self.db,
                                    max_cluster_count,
                                )
                                .and_then(|data| {
                                    let mut clusters: Vec<Clusters> = Vec::new();
                                    for d in data {
                                        let eg = match d.examples {
                                            Some(eg) => {
                                                match rmp_serde::decode::from_slice(&eg)
                                                    as Result<
                                                        Vec<(usize, String)>,
                                                        rmp_serde::decode::Error,
                                                    > {
                                                    Ok(eg) => Some(eg),
                                                    Err(_) => None,
                                                }
                                            }
                                            None => None,
                                        };
                                        clusters.push(Clusters {
                                            cluster_id: d.cluster_id,
                                            examples: eg,
                                        });
                                    }
                                    match serde_json::to_string(&clusters) {
                                        Ok(json) => future::ok(
                                            Response::builder()
                                                .header(header::CONTENT_TYPE, "application/json")
                                                .body(Body::from(json))
                                                .unwrap(),
                                        ),
                                        Err(_) => future::ok(ApiService::build_http_500_response()),
                                    }
                                })
                                .map_err(Into::into);

                                Box::new(result)
                            } else {
                                return Box::new(future::ok(
                                    Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("max_cluster_count must be a positive integer value or 'all'"))
                                        .unwrap(),
                                ));
                            }
                        } else if max_cluster_count == "all" {
                            let result = db::DB::get_cluster(&self.db, cluster_id)
                                .and_then(|data| {
                                    let mut clusters: Vec<Clusters> = Vec::new();
                                    for d in data {
                                        let eg = match d.examples {
                                            Some(eg) => {
                                                match rmp_serde::decode::from_slice(&eg)
                                                    as Result<
                                                        Vec<(usize, String)>,
                                                        rmp_serde::decode::Error,
                                                    > {
                                                    Ok(eg) => Some(eg),
                                                    Err(_) => None,
                                                }
                                            }
                                            None => None,
                                        };
                                        clusters.push(Clusters {
                                            cluster_id: d.cluster_id,
                                            examples: eg,
                                        });
                                    }
                                    match serde_json::to_string(&clusters) {
                                        Ok(json) => future::ok(
                                            Response::builder()
                                                .header(header::CONTENT_TYPE, "application/json")
                                                .body(Body::from(json))
                                                .unwrap(),
                                        ),
                                        Err(_) => future::ok(ApiService::build_http_500_response()),
                                    }
                                })
                                .map_err(Into::into);

                            Box::new(result)
                        } else if let Ok(max_cluster_count) = max_cluster_count.parse::<usize>() {
                            if max_cluster_count == 0 {
                                return Box::new(future::ok(
                                    Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("max_cluster_count must be a positive integer value or 'all'"))
                                        .unwrap(),
                                ));
                            }
                            let result = db::DB::get_cluster_with_limit_num(
                                &self.db,
                                cluster_id,
                                max_cluster_count,
                            )
                            .and_then(|data| {
                                let mut clusters: Vec<Clusters> = Vec::new();
                                for d in data {
                                    let eg = match d.examples {
                                        Some(eg) => {
                                            match rmp_serde::decode::from_slice(&eg)
                                                as Result<
                                                    Vec<(usize, String)>,
                                                    rmp_serde::decode::Error,
                                                > {
                                                Ok(eg) => Some(eg),
                                                Err(_) => None,
                                            }
                                        }
                                        None => None,
                                    };
                                    clusters.push(Clusters {
                                        cluster_id: d.cluster_id,
                                        examples: eg,
                                    });
                                }
                                match serde_json::to_string(&clusters) {
                                    Ok(json) => future::ok(
                                        Response::builder()
                                            .header(header::CONTENT_TYPE, "application/json")
                                            .body(Body::from(json))
                                            .unwrap(),
                                    ),
                                    Err(_) => future::ok(ApiService::build_http_500_response()),
                                }
                            })
                            .map_err(Into::into);

                            Box::new(result)
                        } else {
                            Box::new(future::ok(
                                        Response::builder()
                                            .status(StatusCode::BAD_REQUEST)
                                            .body(Body::from("cluster_id and max_cluster_count must be a positive integer value or 'all'"))
                                            .unwrap(),
                                    ))
                        }
                    } else {
                        Box::new(future::ok(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from("Invalid request"))
                                .unwrap(),
                        ))
                    }
                }
                (&Method::PUT, "/api/cluster") => {
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
                                    "http://{}/api/cluster?qualifier_id={}",
                                    &self.docker_host_addr, benign_id,
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
                                    .and_then(|return_value| {
                                        if return_value != -1 {
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from("Database has been updated"))
                                                    .unwrap(),
                                            )
                                        } else {
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::BAD_REQUEST)
                                                    .body(Body::from("Invalid request"))
                                                    .unwrap(),
                                            )
                                        }
                                    })
                                    .map_err(Into::into);

                            return Box::new(result);
                        }
                    }
                    Box::new(future::ok(
                        Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Invalid request"))
                            .unwrap(),
                    ))
                }
                _ => Box::new(future::ok(ApiService::build_http_404_response())),
            },
            None => match (req.method(), req.uri().path()) {
                (&Method::POST, "/api/cluster") => {
                    #[derive(Debug, Deserialize)]
                    struct Cluster {
                        cluster_id: String,
                        detector_id: i32,
                        signature: Option<String>,
                        data_source: String,
                        size: Option<usize>,
                        examples: Option<Vec<(usize, String)>>,
                    }
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<Cluster>| {
                                    for d in &data {
                                        db::DB::update_cluster(
                                            &self.db,
                                            &d.cluster_id.as_str(),
                                            d.detector_id,
                                            &d.signature,
                                            &d.data_source,
                                            d.size,
                                            &d.examples,
                                        );
                                    }
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|_| {
                            future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Body::from("Cluster information has been updated"))
                                    .unwrap(),
                            )
                        });

                    Box::new(result)
                }
                (&Method::POST, "/api/outlier") => {
                    #[derive(Debug, Deserialize)]
                    struct Outliers {
                        outlier: Vec<u8>,
                        data_source: String,
                        event_ids: Vec<u64>,
                    }
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<Outliers>| {
                                    for d in &data {
                                        db::DB::update_outlier(
                                            &self.db,
                                            &d.outlier,
                                            &d.data_source,
                                            &d.event_ids,
                                        );
                                    }
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|_| {
                            future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Body::from("Outlier information has been updated"))
                                    .unwrap(),
                            )
                        });

                    Box::new(result)
                }
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

                (&Method::GET, "/api/cluster") => {
                    let result = db::DB::get_event_table(&self.db)
                        .and_then(|data| {
                            #[derive(Debug, Serialize)]
                            struct Clusters {
                                cluster_id: Option<String>,
                                detector_id: i32,
                                qualifier_id: i32,
                                status_id: i32,
                                rules: Option<String>,
                                signature: String,
                                data_source: String,
                                size: usize,
                                examples: Option<Vec<(usize, String)>>,
                                last_modification_time: Option<chrono::NaiveDateTime>,
                            }
                            let mut clusters: Vec<Clusters> = Vec::new();
                            for d in data {
                                let eg = match d.examples {
                                    Some(eg) => {
                                        match rmp_serde::decode::from_slice(&eg)
                                            as Result<
                                                Vec<(usize, String)>,
                                                rmp_serde::decode::Error,
                                            > {
                                            Ok(eg) => Some(eg),
                                            Err(_) => None,
                                        }
                                    }
                                    None => None,
                                };
                                let size = d.size.parse::<usize>().unwrap_or(0);
                                clusters.push(Clusters {
                                    cluster_id: d.cluster_id,
                                    detector_id: d.detector_id,
                                    qualifier_id: d.qualifier_id,
                                    status_id: d.status_id,
                                    rules: d.rules,
                                    signature: d.signature,
                                    data_source: d.data_source,
                                    size,
                                    examples: eg,
                                    last_modification_time: d.last_modification_time,
                                });
                            }
                            match serde_json::to_string(&clusters) {
                                Ok(json) => future::ok(
                                    Response::builder()
                                        .header(header::CONTENT_TYPE, "application/json")
                                        .body(Body::from(json))
                                        .unwrap(),
                                ),
                                Err(_) => future::ok(ApiService::build_http_500_response()),
                            }
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }

                (&Method::GET, "/api/outlier") => {
                    let result = db::DB::get_outliers_table(&self.db)
                        .and_then(|data| {
                            #[derive(Debug, Serialize)]
                            struct Outliers {
                                outlier: String,
                                data_source: String,
                                size: usize,
                                event_ids: Vec<u64>,
                            }
                            let mut outliers: Vec<Outliers> = Vec::new();
                            for d in data {
                                let event_ids = match d.outlier_event_ids {
                                    Some(event_ids) => {
                                        match rmp_serde::decode::from_slice(&event_ids)
                                            as Result<Vec<u64>, rmp_serde::decode::Error>
                                        {
                                            Ok(event_ids) => event_ids,
                                            Err(_) => Vec::<u64>::new(),
                                        }
                                    }
                                    None => Vec::<u64>::new(),
                                };
                                let size = match d.outlier_size {
                                    Some(size) => size.parse::<usize>().unwrap_or(0),
                                    None => 0,
                                };
                                outliers.push(Outliers {
                                    outlier: ApiService::bytes_to_string(&d.outlier_raw_event),
                                    data_source: d.outlier_data_source,
                                    size,
                                    event_ids,
                                });
                            }

                            match serde_json::to_string(&outliers) {
                                Ok(json) => future::ok(
                                    Response::builder()
                                        .header(header::CONTENT_TYPE, "application/json")
                                        .body(Body::from(json))
                                        .unwrap(),
                                ),
                                Err(_) => future::ok(ApiService::build_http_500_response()),
                            }
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

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
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
