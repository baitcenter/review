use failure::Fail;
use futures::future;
use futures::future::Future;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::rt::Stream;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use url::percent_encoding::percent_decode;

mod error;
use error::Error;

const SELECT_ALL: db::SelectCluster = (
    true, true, true, true, true, true, true, true, true, true, true,
);

#[derive(Clone)]
pub struct ApiService {
    db: db::DB,
    docker_host_addr: String,
    etcd_url: String,
    reviewd_url: String,
}

impl ApiService {
    pub fn new(
        database_url: &str,
        docker_host_addr: &str,
        etcd_url: &str,
        kafka_url: &str,
    ) -> Box<dyn Future<Item = Self, Error = Error> + Send + 'static> {
        let docker_host_addr = docker_host_addr.to_string();
        let etcd_url = etcd_url.to_string();
        let reviewd_url = database_url.to_string();

        let fut = db::DB::new(database_url, kafka_url.to_string())
            .and_then(move |db| {
                future::ok(Self {
                    db,
                    docker_host_addr,
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
    ) -> Box<dyn Future<Item = Response<Body>, Error = Error> + Send + 'static> {
        match req.uri().query() {
            Some(query) => match (req.method(), req.uri().path()) {
                (&Method::GET, "/api/cluster/search") => {
                    let hash_query: HashMap<_, _> =
                        url::form_urlencoded::parse(query.as_ref()).collect();
                    let where_clause = if let Some(filter) = hash_query.get("filter") {
                        match Filter::get_where_clause(&filter) {
                            Ok(where_clause) => {
                                if where_clause.is_empty() {
                                    None
                                } else {
                                    Some(where_clause)
                                }
                            }
                            Err(_) => {
                                return Box::new(future::ok(ApiService::build_http_400_response()))
                            }
                        }
                    } else {
                        return Box::new(future::ok(ApiService::build_http_400_response()));
                    };
                    if hash_query.len() == 1 {
                        let result = db::DB::execute_select_cluster_query(
                            &self.db,
                            where_clause,
                            None,
                            SELECT_ALL,
                        )
                        .and_then(|(data, return_score)| {
                            future::ok(ApiService::build_cluster_response(data, return_score))
                        })
                        .map_err(Into::into);
                        return Box::new(result);
                    } else if let (Some(limit), 2) = (hash_query.get("limit"), hash_query.len()) {
                        if let Ok(limit) = limit.parse::<u64>() {
                            let result = db::DB::execute_select_cluster_query(
                                &self.db,
                                where_clause,
                                Some(limit as i64),
                                SELECT_ALL,
                            )
                            .and_then(|(data, return_score)| {
                                future::ok(ApiService::build_cluster_response(data, return_score))
                            })
                            .map_err(Into::into);
                            return Box::new(result);
                        }
                    } else if let (Some(select), 2) = (hash_query.get("select"), hash_query.len()) {
                        if let Ok(select) = serde_json::from_str::<Select>(&select) {
                            let select = Select::response_type_builder(&select);
                            let result = db::DB::execute_select_cluster_query(
                                &self.db,
                                where_clause,
                                None,
                                select,
                            )
                            .and_then(|(data, return_score)| {
                                future::ok(ApiService::build_cluster_response(data, return_score))
                            })
                            .map_err(Into::into);
                            return Box::new(result);
                        } else {
                            return Box::new(future::ok(ApiService::build_http_400_response()));
                        }
                    } else if let (Some(select), Some(limit), 3) = (
                        hash_query.get("select"),
                        hash_query.get("limit"),
                        hash_query.len(),
                    ) {
                        if let (Ok(select), Ok(limit)) = (
                            serde_json::from_str::<Select>(&select),
                            limit.parse::<u64>(),
                        ) {
                            let select = Select::response_type_builder(&select);
                            let result = db::DB::execute_select_cluster_query(
                                &self.db,
                                where_clause,
                                Some(limit as i64),
                                select,
                            )
                            .and_then(|(data, return_score)| {
                                future::ok(ApiService::build_cluster_response(data, return_score))
                            })
                            .map_err(Into::into);
                            return Box::new(result);
                        } else {
                            return Box::new(future::ok(ApiService::build_http_400_response()));
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_404_response()))
                }
                (&Method::POST, "/api/category") => {
                    let query = url::form_urlencoded::parse(query.as_ref()).collect::<Vec<_>>();
                    if query.len() == 1 && query[0].0 == "category" {
                        let resp =
                            db::DB::add_category(&self.db, &query[0].1).then(|insert_result| {
                                match insert_result {
                                    Ok(_) => future::ok(
                                        Response::builder()
                                            .status(StatusCode::CREATED)
                                            .body(Body::from("New category has been added"))
                                            .expect("builder with known status code must not fail"),
                                    ),
                                    Err(e) => future::ok(ApiService::db_error_handler(&e)),
                                }
                            });
                        return Box::new(resp);
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::PUT, "/api/raw_events") => {
                    let hash_query: HashMap<_, _> =
                        url::form_urlencoded::parse(query.as_ref()).collect();
                    if let (Some(data_source), Some(max_event_count), 2) = (
                        hash_query.get("data_source"),
                        hash_query.get("max_event_count"),
                        hash_query.len(),
                    ) {
                        if let Ok(max_event_count) = max_event_count.parse::<usize>() {
                            let resp = db::DB::add_raw_events(
                                &self.db,
                                &data_source,
                                max_event_count,
                            )
                            .and_then(|add_result| {
                                if add_result != 0 {
                                    future::ok(
                                        Response::builder()
                                            .status(StatusCode::CREATED)
                                            .body(Body::from("RawEvent table have been updated"))
                                            .expect("builder with known status code must not fail"),
                                    )
                                } else {
                                    future::ok(ApiService::build_http_500_response())
                                }
                            })
                            .map_err(Into::into);

                            return Box::new(resp);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::PUT, "/api/etcd/suspicious_tokens") => {
                    let query = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect::<Vec<_>>();
                    if query.len() == 1 && query[0].0 == "etcd_key" {
                        let result =
                            req.into_body()
                                .concat2()
                                .map_err(Into::into)
                                .and_then(move |buf| {
                                    let data = format!(
                                        r#"{{"key": "{}", "value": "{}"}}"#,
                                        base64::encode(&query[0].1),
                                        base64::encode(&buf)
                                    );
                                    let client = reqwest::Client::new();
                                    match client.post(&self.etcd_url).body(data).send() {
                                        Ok(_) => {
                                            let msg = format!("{} has been updated.", &query[0].1);
                                            future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from(msg))
                                                .expect(
                                                    "builder with known status code must not fail",
                                                ),
                                        )
                                        }
                                        Err(e) => {
                                            let err_msg = format!(
                                                "An error occurs while updating etcd value: {}",
                                                e
                                            );
                                            eprintln!("{}", err_msg);
                                            future::ok(ApiService::build_http_response(
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                                &err_msg,
                                            ))
                                        }
                                    }
                                });
                        return Box::new(result);
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                _ => {
                    if req.method() == Method::PUT && req.uri().path().starts_with("/api/cluster/")
                    {
                        let path: Vec<&str> = req.uri().path().split('/').collect();
                        let query = url::form_urlencoded::parse(query.as_ref())
                            .into_owned()
                            .collect::<Vec<_>>();
                        if path.len() == 4 && query.len() == 1 && query[0].0 == "data_source" {
                            let cluster_id = percent_decode(path[3].as_bytes()).decode_utf8();
                            if let Ok(cluster_id) = cluster_id {
                                #[derive(Debug, Deserialize)]
                                struct NewValues {
                                    cluster_id: Option<String>,
                                    category: Option<String>,
                                    qualifier: Option<String>,
                                }
                                let cluster_id_cloned = cluster_id.into_owned();
                                let data_source_cloned = query[0].1.clone();
                                let docker_host_addr = self.docker_host_addr.clone();
                                let etcd_url = self.etcd_url.clone();
                                let result = req
                                    .into_body()
                                    .concat2()
                                    .map_err(Into::into)
                                    .and_then(|buf| {
                                        serde_json::from_slice(&buf)
                                            .map(move |data: NewValues| {
                                                if data.cluster_id.is_some() || data.category.is_some() || data.qualifier.is_some() {
                                                    db::DB::update_cluster(&self.db, &cluster_id_cloned, &data_source_cloned, data.cluster_id, data.category, data.qualifier, )
                                                }
                                                else {
                                                    future::result(Err(db::error::Error::from(db::error::ErrorKind::DatabaseTransactionError(
                                                                        db::error::DatabaseError::RecordNotExist,
                                                                    ))))
                                                }
                                            })
                                            .map_err(Into::into)
                                    })
                                    .and_then(move |mut result| match result.poll() {
                                        Ok(fut) => {
                                            if let futures::prelude::Async::Ready((row, is_benign, data_source)) = fut {
                                                if row != 1 {
                                                    return future::ok(ApiService::build_http_response(
                                                        StatusCode::BAD_REQUEST,
                                                        "The specified record does not exist in database",
                                                    ));
                                                } else if is_benign {
                                                    let etcd_value = format!(
                                                        r#"http://{}/api/cluster/search?filter={{"qualifier": ["benign"], "data_source":["{}"]}}"#,
                                                        &docker_host_addr, data_source
                                                    );
                                                    let etcd_key = format!("benign_signatures_{}", &data_source);
                                                    ApiService::update_etcd(&etcd_url, &etcd_key, &etcd_value);
                                                }
                                            }
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from("Cluster has been successfully updated"))
                                                    .expect("builder with known status code must not fail"),
                                            )
                                        }
                                        Err(e) => future::ok(ApiService::db_error_handler(&e)),
                                    });

                                return Box::new(result);
                            }
                        }
                    }

                    Box::new(future::ok(ApiService::build_http_404_response()))
                }
            },
            None => match (req.method(), req.uri().path()) {
                (&Method::POST, "/api/cluster") => {
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<db::models::ClusterUpdate>| {
                                    db::DB::add_clusters(&self.db, &data)
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|mut result| match result.poll() {
                            Ok(_) => future::ok(
                                Response::builder()
                                    .status(StatusCode::CREATED)
                                    .body(Body::from(
                                        "New clusters have been inserted into database",
                                    ))
                                    .expect("builder with known status code must not fail"),
                            ),
                            Err(e) => future::ok(ApiService::db_error_handler(&e)),
                        });

                    Box::new(result)
                }
                (&Method::POST, "/api/outlier") => {
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<db::models::OutlierUpdate>| {
                                    db::DB::add_outliers(&self.db, &data)
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|_| {
                            future::ok(
                                Response::builder()
                                    .status(StatusCode::CREATED)
                                    .body(Body::from(
                                        "New outliers have been inserted into database",
                                    ))
                                    .expect("builder with known status code must not fail"),
                            )
                        });

                    Box::new(result)
                }
                (&Method::PUT, "/api/cluster") => {
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<db::models::ClusterUpdate>| {
                                    db::DB::update_clusters(&self.db, &data)
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|mut result| match result.poll() {
                            Ok(_) => future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Body::from("Clusters have been updated"))
                                    .expect("builder with known status code must not fail"),
                            ),
                            Err(e) => future::ok(ApiService::db_error_handler(&e)),
                        });

                    Box::new(result)
                }
                (&Method::PUT, "/api/outlier") => {
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(|buf| {
                            serde_json::from_slice(&buf)
                                .map(move |data: Vec<db::models::OutlierUpdate>| {
                                    db::DB::update_outliers(&self.db, &data)
                                })
                                .map_err(Into::into)
                        })
                        .and_then(|mut result| match result.poll() {
                            Ok(_) => future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Body::from("Outlier information has been updated"))
                                    .expect("builder with known status code must not fail"),
                            ),
                            Err(e) => future::ok(ApiService::db_error_handler(&e)),
                        });

                    Box::new(result)
                }
                (&Method::GET, "/api/category") => {
                    let result = db::DB::get_category_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .expect("builder with known status code must not fail"),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }
                (&Method::GET, "/api/cluster") => {
                    let result = db::DB::get_cluster_table(&self.db)
                        .and_then(|data| future::ok(ApiService::build_cluster_response(data, true)))
                        .map_err(Into::into);
                    Box::new(result)
                }
                (&Method::PUT, "/api/cluster/qualifier") => {
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(move |buf| {
                            match serde_json::from_slice::<Vec<db::models::QualifierUpdate>>(&buf) {
                                Ok(data) => {
                                    let mut result = db::DB::update_qualifiers(&self.db, &data);
                                    match result.poll() {
                                        Ok(_) => {
                                            data.iter().for_each(|d| {
                                                if d.qualifier == "benign" {
                                                    let etcd_value = format!(
                                                        r#"http://{}/api/cluster/search?filter={{"qualifier": ["benign"], "data_source":["{}"]}}"#,
                                                        &self.docker_host_addr, &d.data_source
                                                    );
                                                    let etcd_key = format!("benign_signatures_{}", &d.data_source);
                                                    ApiService::update_etcd(&self.etcd_url, &etcd_key, &etcd_value);
                                                }
                                            });
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from("Qualifier has been successfully updated"))
                                                    .expect("builder with known status code must not fail"),
                                            )
                                        }
                                        Err(e) => future::ok(ApiService::db_error_handler(&e)),
                                    }

                                }
                                Err(_) => future::ok(
                                    ApiService::build_http_response(StatusCode::BAD_REQUEST, "Invalid JSON format")
                                ),
                            }
                        });

                    Box::new(result)
                }
                (&Method::GET, "/api/outlier") => {
                    let result = db::DB::get_outliers_table(&self.db)
                        .and_then(|data| future::ok(ApiService::process_outliers(data)))
                        .map_err(Into::into);

                    Box::new(result)
                }
                (&Method::GET, "/api/qualifier") => {
                    let result = db::DB::get_qualifier_table(&self.db)
                        .and_then(|data| match serde_json::to_string(&data) {
                            Ok(json) => future::ok(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .expect("builder with known status code must not fail"),
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
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(json))
                                    .expect("builder with known status code must not fail"),
                            ),
                            Err(_) => future::ok(ApiService::build_http_500_response()),
                        })
                        .map_err(Into::into);

                    Box::new(result)
                }
                _ => {
                    if req.method() == Method::GET && req.uri().path().starts_with("/api/outlier/")
                    {
                        let path: Vec<&str> = req.uri().path().split('/').collect();
                        if path.len() == 4 {
                            let data_source = percent_decode(path[3].as_bytes()).decode_utf8();
                            if let Ok(data_source) = data_source {
                                let result =
                                    db::DB::execute_select_outlier_query(&self.db, &data_source)
                                        .and_then(|data| {
                                            future::ok(ApiService::process_outliers(data))
                                        })
                                        .map_err(Into::into);

                                return Box::new(result);
                            }
                        }
                    } else if req.method() == Method::PUT
                        && req.uri().path().starts_with("/api/category/")
                    {
                        let path: Vec<&str> = req.uri().path().split('/').collect();
                        if path.len() == 4 {
                            let category = percent_decode(path[3].as_bytes()).decode_utf8();
                            if let Ok(category) = category {
                                #[derive(Debug, Deserialize)]
                                struct NewCategory {
                                    category: String,
                                }
                                let category_cloned = category.into_owned();
                                let result = req
                                    .into_body()
                                    .concat2()
                                    .map_err(Into::into)
                                    .and_then(|buf| {
                                        serde_json::from_slice(&buf)
                                            .map(move |new_category: NewCategory| {
                                                db::DB::update_category(
                                                    &self.db,
                                                    &category_cloned,
                                                    &new_category.category,
                                                )
                                            })
                                            .map_err(Into::into)
                                    })
                                    .and_then(|mut result| match result.poll() {
                                        Ok(_) => future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from(
                                                    "Category has been successfully updated",
                                                ))
                                                .expect(
                                                    "builder with known status code must not fail",
                                                ),
                                        ),
                                        Err(e) => future::ok(ApiService::db_error_handler(&e)),
                                    });

                                return Box::new(result);
                            }
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_404_response()))
                }
            },
        }
    }

    pub fn api_error_handler(
        res_body: Result<Response<Body>, Error>,
    ) -> Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send + 'static> {
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
                    .expect("builder with known status code must not fail");

                Box::new(future::ok(res.map(Into::into)))
            }
        }
    }

    fn db_error_handler(e: &db::error::Error) -> Response<Body> {
        if let Some(e) = e.cause() {
            let cause = e.find_root_cause().to_string();
            if cause.contains("database is locked") {
                ApiService::build_http_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Service temporarily unavailable",
                )
            } else if cause.contains("NotFound") {
                ApiService::build_http_response(
                    StatusCode::BAD_REQUEST,
                    "The specified record does not exist in database",
                )
            } else {
                ApiService::build_http_500_response()
            }
        } else if let db::error::ErrorKind::DatabaseTransactionError(reason) = e.kind() {
            match *reason {
                db::error::DatabaseError::DatabaseLocked => ApiService::build_http_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Service temporarily unavailable",
                ),
                db::error::DatabaseError::RecordNotExist => ApiService::build_http_response(
                    StatusCode::BAD_REQUEST,
                    "The specified record does not exist in database",
                ),
                db::error::DatabaseError::Other => ApiService::build_http_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Please make sure that the values in your request are correct",
                ),
            }
        } else {
            ApiService::build_http_500_response()
        }
    }

    fn build_http_response(status_code: http::status::StatusCode, message: &str) -> Response<Body> {
        let body = json!({
            "message": message,
        })
        .to_string();
        Response::builder()
            .status(status_code)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, body.len().to_string().as_str())
            .body(body.into())
            .expect("builder with known status code must not fail")
    }

    fn build_http_400_response() -> Response<Body> {
        let body = json!({
            "message": "Invalid request",
        })
        .to_string();
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, body.len().to_string().as_str())
            .body(body.into())
            .expect("builder with known status code must not fail")
    }

    fn build_http_404_response() -> Response<Body> {
        let body = json!({
            "message": "Not found",
        })
        .to_string();
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, body.len().to_string().as_str())
            .body(body.into())
            .expect("builder with known status code must not fail")
    }

    fn build_http_500_response() -> Response<Body> {
        let body = json!({
            "message": "Internal server error",
        })
        .to_string();
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, body.len().to_string().as_str())
            .body(body.into())
            .expect("builder with known status code must not fail")
    }

    fn build_cluster_response(
        data: Vec<db::ClusterResponse>,
        return_score: bool,
    ) -> Response<Body> {
        let mut json = String::new();
        json.push_str("[");
        for (index, d) in data.iter().enumerate() {
            json.push_str("{");
            let mut j = String::new();
            if let Some(cluster_id) = &d.0 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "cluster_id",
                    cluster_id,
                ));
            }
            if let Some(detector_id) = d.1 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "detector_id",
                    detector_id,
                ));
            }
            if let Some(qualifier) = &d.2 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "qualifier",
                    qualifier,
                ));
            }
            if let Some(status) = &d.3 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "status",
                    status,
                ));
            }
            if let Some(category) = &d.4 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "category",
                    category,
                ));
            }
            if let Some(signature) = &d.5 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "signature",
                    signature,
                ));
            }
            if let Some(data_source) = &d.6 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "data_source",
                    data_source,
                ));
            }
            if let Some(size) = &d.7 {
                j.push_str(&ApiService::build_response_string(
                    j.is_empty(),
                    "size",
                    size,
                ));
            }
            if return_score {
                if let Some(score) = d.8 {
                    j.push_str(&ApiService::build_response_string(
                        j.is_empty(),
                        "score",
                        score,
                    ));
                } else {
                    j.push_str(&ApiService::build_response_string(
                        j.is_empty(),
                        "score",
                        "-",
                    ));
                }
            }
            if let Some(examples) = &d.9 {
                match serde_json::to_string(&examples) {
                    Ok(e) => {
                        if !j.is_empty() {
                            j.push_str(&format!(r#","examples":{}"#, e));
                        } else {
                            j.push_str(&format!(r#""examples":{}"#, e));
                        }
                    }
                    Err(_) => {
                        if !j.is_empty() {
                            j.push_str(r#","examples":-"#);
                        } else {
                            j.push_str(r#""examples":-"#);
                        }
                    }
                }
            }
            if let Some(last_modification_time) = &d.10 {
                if !j.is_empty() {
                    j.push_str(&format!(
                        r#","last_modification_time":"{}""#,
                        last_modification_time
                    ));
                } else {
                    j.push_str(&format!(
                        r#""last_modification_time":"{}""#,
                        last_modification_time
                    ));
                }
            }
            if index == data.len() - 1 {
                j.push_str("}")
            } else {
                j.push_str("},")
            }

            json.push_str(&j);
        }
        json.push_str("]");
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json))
            .expect("builder with known status code must not fail")
    }

    fn build_response_string<T: std::fmt::Debug>(
        is_first_property: bool,
        property_name: &str,
        property_value: T,
    ) -> String {
        if is_first_property {
            format!(r#""{}":{:?}"#, property_name, property_value)
        } else {
            format!(r#","{}":{:?}"#, property_name, property_value)
        }
    }

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
    }

    fn process_outliers(
        data: Vec<(db::models::OutliersTable, db::models::DataSourceTable)>,
    ) -> Response<Body> {
        #[derive(Debug, Serialize)]
        struct Outliers {
            outlier: String,
            data_source: String,
            size: usize,
            event_ids: Vec<u64>,
        }
        let mut outliers: Vec<Outliers> = Vec::new();
        for d in data {
            let event_ids = d.0.event_ids.map_or(Vec::<u64>::new(), |event_ids| {
                rmp_serde::decode::from_slice::<Vec<u64>>(&event_ids).unwrap_or_default()
            });
            let size =
                d.0.size
                    .map_or(0, |size| size.parse::<usize>().unwrap_or(0));
            outliers.push(Outliers {
                outlier: ApiService::bytes_to_string(&d.0.raw_event),
                data_source: d.1.topic_name,
                size,
                event_ids,
            });
        }

        match serde_json::to_string(&outliers) {
            Ok(json) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .expect("builder with known status code must not fail"),
            Err(_) => ApiService::build_http_500_response(),
        }
    }

    fn update_etcd(url: &str, key: &str, value: &str) {
        let data = format!(
            r#"{{"key": "{}", "value": "{}"}}"#,
            base64::encode(key),
            base64::encode(value)
        );
        let client = reqwest::Client::new();
        if let Err(e) = client.post(url).body(data).send() {
            eprintln!("An error occurs while updating etcd: {}", e);
        }
    }
}

#[derive(Debug, Deserialize)]
struct Filter {
    category: Option<Vec<String>>,
    data_source: Option<Vec<String>>,
    status: Option<Vec<String>>,
    qualifier: Option<Vec<String>>,
    cluster_id: Option<Vec<String>>,
    detector_id: Option<Vec<u64>>,
}

impl Filter {
    fn query_builder(&self) -> Vec<String> {
        let mut query = Vec::<String>::new();
        if let Some(category) = &self.category {
            query.extend(category.iter().map(|c| {
                format!(
                    "Clusters.category_id = (SELECT category_id FROM category WHERE category = '{}')",
                    c
                )
            }));
        }
        let query = match &self.cluster_id {
            Some(cluster_id) => {
                let cluster_id = cluster_id
                    .iter()
                    .map(|c| format!("cluster_id='{}'", c))
                    .collect::<Vec<String>>();
                Filter::build_where_clause(&query, &cluster_id)
            }
            None => query,
        };
        let query = match &self.data_source {
            Some(data_source) => {
                let data_source = data_source
                    .iter()
                    .map(|d| format!("Clusters.data_source_id = (SELECT data_source_id FROM DataSource WHERE topic_name = '{}')", d))
                    .collect::<Vec<String>>();
                Filter::build_where_clause(&query, &data_source)
            }
            None => query,
        };
        let query = match &self.detector_id {
            Some(detector_id) => {
                let detector_id = detector_id
                    .iter()
                    .map(|d| format!("detector_id='{}'", d))
                    .collect::<Vec<String>>();
                Filter::build_where_clause(&query, &detector_id)
            }
            None => query,
        };
        let query = match &self.status {
            Some(status) => {
                let status = status
                    .iter()
                    .map(|s| {
                        format!(
                            "Clusters.status_id = (SELECT status_id FROM status WHERE status = '{}')",
                            s
                        )
                    })
                    .collect::<Vec<String>>();
                Filter::build_where_clause(&query, &status)
            }
            None => query,
        };
        match &self.qualifier {
            Some(qualifier) => {
                let qualifier = qualifier.iter().map(|q| format!("Clusters.qualifier_id = (SELECT qualifier_id FROM qualifier WHERE qualifier = '{}')", q)).collect::<Vec<String>>();
                Filter::build_where_clause(&query, &qualifier)
            }
            None => query,
        }
    }

    fn build_where_clause(query: &[String], new_filters: &[String]) -> Vec<String> {
        if query.is_empty() {
            new_filters
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        } else {
            let mut new_query = Vec::<String>::new();
            for q in query {
                new_query.extend(new_filters.iter().map(|f| format!("{} and {}", q, f)));
            }
            new_query
        }
    }

    fn get_where_clause(filter: &str) -> Result<String, Error> {
        serde_json::from_str(filter)
            .map_err(Into::into)
            .and_then(|filter: Filter| {
                let filter = Filter::query_builder(&filter);
                let mut where_clause = String::new();
                for (index, filter) in filter.iter().enumerate() {
                    if index == 0 {
                        where_clause.push_str(&filter);
                    } else {
                        where_clause.push_str(&format!(" or {}", filter))
                    }
                }
                Ok(where_clause)
            })
    }
}

#[derive(Debug, Deserialize)]
struct Select {
    cluster_id: Option<bool>,
    detector_id: Option<bool>,
    qualifier: Option<bool>,
    status: Option<bool>,
    category: Option<bool>,
    signature: Option<bool>,
    data_source: Option<bool>,
    size: Option<bool>,
    score: Option<bool>,
    examples: Option<bool>,
    last_modification_time: Option<bool>,
}

impl Select {
    fn response_type_builder(&self) -> db::SelectCluster {
        (
            self.cluster_id.unwrap_or_else(|| false),
            self.detector_id.unwrap_or_else(|| false),
            self.qualifier.unwrap_or_else(|| false),
            self.status.unwrap_or_else(|| false),
            self.category.unwrap_or_else(|| false),
            self.signature.unwrap_or_else(|| false),
            self.data_source.unwrap_or_else(|| false),
            self.size.unwrap_or_else(|| false),
            self.score.unwrap_or_else(|| false),
            self.examples.unwrap_or_else(|| false),
            self.last_modification_time.unwrap_or_else(|| false),
        )
    }
}
