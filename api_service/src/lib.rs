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

    #[allow(clippy::cyclomatic_complexity)]
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
                    if hash_query.len() == 1 {
                        if let Some(category_id) = hash_query.get("category_id") {
                            let result = db::DB::get_cluster_by_category(&self.db, category_id)
                                .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                .map_err(Into::into);
                            Box::new(result)
                        } else if let Some(data_source) = hash_query.get("data_source") {
                            let result = db::DB::get_cluster_by_data_source(&self.db, data_source)
                                .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                .map_err(Into::into);
                            Box::new(result)
                        } else if let Some(status_id) = hash_query.get("status_id") {
                            if let Ok(status_id) = status_id.parse::<i32>() {
                                let result = db::DB::get_cluster_by_status(&self.db, status_id)
                                    .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                    .map_err(Into::into);
                                Box::new(result)
                            } else {
                                Box::new(future::ok(ApiService::build_http_404_response()))
                            }
                        } else if let Some(qualifier_id) = hash_query.get("qualifier_id") {
                            if let Ok(qualifier_id) = qualifier_id.parse::<i32>() {
                                let result =
                                    db::DB::get_signature_by_qualifier(&self.db, qualifier_id)
                                        .and_then(|data| match serde_json::to_string(&data) {
                                            Ok(json) => future::ok(
                                                Response::builder()
                                                    .header(
                                                        header::CONTENT_TYPE,
                                                        "application/json",
                                                    )
                                                    .body(Body::from(json))
                                                    .unwrap(),
                                            ),
                                            Err(_) => {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        })
                                        .map_err(Into::into);

                                Box::new(result)
                            } else {
                                Box::new(future::ok(ApiService::build_http_400_response()))
                            }
                        } else {
                            Box::new(future::ok(ApiService::build_http_400_response()))
                        }
                    } else if hash_query.len() == 2 {
                        if let (Some(cluster_id), Some(max_cluster_count)) = (
                            hash_query.get("cluster_id"),
                            hash_query.get("max_cluster_count"),
                        ) {
                            #[derive(Debug, Serialize)]
                            struct Clusters {
                                cluster_id: Option<String>,
                                examples: Option<Vec<(usize, String)>>,
                            }
                            if cluster_id == "all" && max_cluster_count == "all" {
                                Box::new(future::ok(ApiService::build_http_400_response()))
                            } else if cluster_id == "all" {
                                if let Ok(max_cluster_count) = max_cluster_count.parse::<usize>() {
                                    if max_cluster_count == 0 {
                                        return Box::new(future::ok(
                                            ApiService::build_http_response(StatusCode::BAD_REQUEST, "max_cluster_count must be a positive integer value or 'all'")
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
                                                Some(eg) => (rmp_serde::decode::from_slice(&eg)
                                                    as Result<
                                                        Vec<(usize, String)>,
                                                        rmp_serde::decode::Error,
                                                    >)
                                                    .ok(),
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
                                                    .header(
                                                        header::CONTENT_TYPE,
                                                        "application/json",
                                                    )
                                                    .body(Body::from(json))
                                                    .unwrap(),
                                            ),
                                            Err(_) => {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
                                    })
                                    .map_err(Into::into);

                                    Box::new(result)
                                } else {
                                    return Box::new(future::ok(
                                        ApiService::build_http_response(StatusCode::BAD_REQUEST, "max_cluster_count must be a positive integer value or 'all'")
                                    ));
                                }
                            } else if max_cluster_count == "all" {
                                let result = db::DB::get_cluster(&self.db, cluster_id)
                                    .and_then(|data| {
                                        let mut clusters: Vec<Clusters> = Vec::new();
                                        for d in data {
                                            let eg = match d.examples {
                                                Some(eg) => (rmp_serde::decode::from_slice(&eg)
                                                    as Result<
                                                        Vec<(usize, String)>,
                                                        rmp_serde::decode::Error,
                                                    >)
                                                    .ok(),
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
                                                    .header(
                                                        header::CONTENT_TYPE,
                                                        "application/json",
                                                    )
                                                    .body(Body::from(json))
                                                    .unwrap(),
                                            ),
                                            Err(_) => {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
                                    })
                                    .map_err(Into::into);

                                Box::new(result)
                            } else if let Ok(max_cluster_count) = max_cluster_count.parse::<usize>()
                            {
                                if max_cluster_count == 0 {
                                    return Box::new(future::ok(
                                        ApiService::build_http_response(StatusCode::BAD_REQUEST, "max_cluster_count must be a positive integer value or 'all'")
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
                                            Some(eg) => (rmp_serde::decode::from_slice(&eg)
                                                as Result<
                                                    Vec<(usize, String)>,
                                                    rmp_serde::decode::Error,
                                                >)
                                                .ok(),
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
                                    ApiService::build_http_response(StatusCode::BAD_REQUEST, "cluster_id and max_cluster_count must be a positive integer value or 'all'")
                                        ))
                            }
                        } else if let (Some(data_source), Some(cluster_only)) = (
                            hash_query.get("data_source"),
                            hash_query.get("cluster_only"),
                        ) {
                            if cluster_only == "true" {
                                let result = db::DB::get_cluster_only(&self.db, data_source)
                                    .and_then(|data| {
                                        let data =
                                            data.iter().filter(|d| d.is_some()).collect::<Vec<_>>();
                                        match serde_json::to_string(&data) {
                                            Ok(json) => future::ok(
                                                Response::builder()
                                                    .header(
                                                        header::CONTENT_TYPE,
                                                        "application/json",
                                                    )
                                                    .body(Body::from(json))
                                                    .unwrap(),
                                            ),
                                            Err(_) => {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
                                    })
                                    .map_err(Into::into);

                                Box::new(result)
                            } else {
                                let result =
                                    db::DB::get_cluster_by_data_source(&self.db, data_source)
                                        .and_then(|data| {
                                            future::ok(ApiService::process_clusters(data))
                                        })
                                        .map_err(Into::into);
                                Box::new(result)
                            }
                        } else {
                            Box::new(future::ok(ApiService::build_http_400_response()))
                        }
                    } else {
                        Box::new(future::ok(ApiService::build_http_400_response()))
                    }
                }
                (&Method::GET, "/api/cluster/example") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if let (Some(limit), 1) = (hash_query.get("limit"), hash_query.len()) {
                        if let Ok(limit) = limit.parse::<u64>() {
                            let query = format!(
                                "SELECT cluster_id, examples FROM Clusters LIMIT {};",
                                limit
                            );
                            let result = db::DB::get_cluster_examples(&self.db, &query)
                                .and_then(|data| {
                                    future::ok(ApiService::process_cluster_examples(data))
                                })
                                .map_err(Into::into);
                            return Box::new(result);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::GET, "/api/cluster/search") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if let (Some(filter), 1) = (hash_query.get("filter"), hash_query.len()) {
                        if let Ok(filter) = serde_json::from_str(&filter)
                            as Result<Filter, serde_json::error::Error>
                        {
                            let filter = Filter::query_builder(&filter);
                            if !filter.is_empty() {
                                let mut where_clause = String::new();
                                for (index, filter) in filter.iter().enumerate() {
                                    if index == 0 {
                                        where_clause.push_str(&filter);
                                    } else {
                                        where_clause.push_str(&format!(" or {}", filter));
                                    }
                                }
                                let query = format!("SELECT * FROM Clusters INNER JOIN Category ON Clusters.category_id = Category.category_id INNER JOIN Qualifier ON Clusters.qualifier_id = Qualifier.qualifier_id INNER JOIN Status ON Clusters.status_id = Status.status_id WHERE {};", where_clause);
                                let result = db::DB::get_cluster_by_filter(&self.db, &query)
                                    .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                    .map_err(Into::into);
                                return Box::new(result);
                            } else {
                                let result = db::DB::get_cluster_table(&self.db)
                                    .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                    .map_err(Into::into);
                                return Box::new(result);
                            }
                        }
                    } else if let (Some(filter), Some(limit), 2) = (
                        hash_query.get("filter"),
                        hash_query.get("limit"),
                        hash_query.len(),
                    ) {
                        if let (Ok(filter), Ok(limit)) = (
                            serde_json::from_str(&filter)
                                as Result<Filter, serde_json::error::Error>,
                            limit.parse::<u64>(),
                        ) {
                            let filter = Filter::query_builder(&filter);
                            if !filter.is_empty() {
                                let mut where_clause = String::new();
                                for (index, filter) in filter.iter().enumerate() {
                                    if index == 0 {
                                        where_clause.push_str(&filter);
                                    } else {
                                        where_clause.push_str(&format!(" or {}", filter));
                                    }
                                }
                                let query = format!("SELECT * FROM Clusters INNER JOIN Category ON Clusters.category_id = Category.category_id INNER JOIN Qualifier ON Clusters.qualifier_id = Qualifier.qualifier_id INNER JOIN Status ON Clusters.status_id = Status.status_id WHERE {} LIMIT {};", where_clause, limit);
                                let result = db::DB::get_cluster_by_filter(&self.db, &query)
                                    .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                    .map_err(Into::into);
                                return Box::new(result);
                            } else {
                                let query = format!("SELECT * FROM Clusters INNER JOIN Category ON Clusters.category_id = Category.category_id INNER JOIN Qualifier ON Clusters.qualifier_id = Qualifier.qualifier_id INNER JOIN Status ON Clusters.status_id = Status.status_id LIMIT {};", limit);
                                let result = db::DB::get_cluster_by_filter(&self.db, &query)
                                    .and_then(|data| future::ok(ApiService::process_clusters(data)))
                                    .map_err(Into::into);
                                return Box::new(result);
                            }
                        }
                    }

                    Box::new(future::ok(ApiService::build_http_404_response()))
                }
                (&Method::GET, "/api/outlier") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if hash_query.len() == 2 {
                        if let (Some(data_source), Some(outlier_only)) = (
                            hash_query.get("data_source"),
                            hash_query.get("outlier_only"),
                        ) {
                            if outlier_only == "true" {
                                let result = db::DB::get_outlier_only(&self.db, data_source)
                                    .and_then(|data| {
                                        let data = data
                                            .iter()
                                            .map(|d| ApiService::bytes_to_string(&d))
                                            .collect::<Vec<_>>();
                                        match serde_json::to_string(&data) {
                                            Ok(json) => future::ok(
                                                Response::builder()
                                                    .header(
                                                        header::CONTENT_TYPE,
                                                        "application/json",
                                                    )
                                                    .body(Body::from(json))
                                                    .unwrap(),
                                            ),
                                            Err(_) => {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
                                    })
                                    .map_err(Into::into);

                                return Box::new(result);
                            }
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::POST, "/api/category") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if hash_query.len() == 1 {
                        if let Some(category) = hash_query.get("category") {
                            let resp = db::DB::add_new_category(&self.db, &category).then(
                                |insert_result| match insert_result {
                                    Ok(_) => future::ok(
                                        Response::builder()
                                            .status(StatusCode::CREATED)
                                            .body(Body::from("New category has been added"))
                                            .unwrap(),
                                    ),
                                    Err(err) => {
                                        let is_temporary_error =
                                            if let db::error::ErrorKind::DatabaseTransactionError(
                                                reason,
                                            ) = err.kind()
                                            {
                                                *reason == db::error::DatabaseError::DatabaseLocked
                                            } else {
                                                false
                                            };
                                        if is_temporary_error {
                                            future::ok(ApiService::build_http_response(
                                                StatusCode::SERVICE_UNAVAILABLE,
                                                "Service temporarily unavailable",
                                            ))
                                        } else {
                                            future::ok(ApiService::build_http_500_response())
                                        }
                                    }
                                },
                            );
                            return Box::new(resp);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::PUT, "/api/category") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if hash_query.len() == 2 {
                        if let (Some(category), Some(new_category)) =
                            (hash_query.get("category"), hash_query.get("new_category"))
                        {
                            let resp = db::DB::update_category(&self.db, &category, &new_category)
                                .then(move |update_result| match update_result {
                                    Ok(_) => future::ok(
                                        Response::builder()
                                            .status(StatusCode::OK)
                                            .body(Body::from("The category has been updated"))
                                            .unwrap(),
                                    ),
                                    Err(err) => {
                                        if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                            err.kind()
                                        {
                                            if *reason == db::error::DatabaseError::DatabaseLocked {
                                                return future::ok(
                                                    ApiService::build_http_response(StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable")
                                                );
                                            } else if *reason
                                                == db::error::DatabaseError::RecordNotExist
                                            {
                                                return future::ok(
                                                    ApiService::build_http_response(StatusCode::BAD_REQUEST, "The specified category does not exist in database")
                                                );
                                            }
                                        }
                                        future::ok(ApiService::build_http_500_response())
                                    }
                                });
                            return Box::new(resp);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::PUT, "/api/cluster") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if hash_query.len() == 2 {
                        if let (Some(cluster_id), Some(qualifier_id)) =
                            (hash_query.get("cluster_id"), hash_query.get("qualifier_id"))
                        {
                            if let Ok(qualifier_id) = qualifier_id.parse::<i32>() {
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
                                let result = db::DB::update_qualifier_id(
                                    &self.db,
                                    &cluster_id,
                                    qualifier_id,
                                )
                                .and_then(|return_value| {
                                    if return_value != -1 {
                                        future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from("Database has been updated"))
                                                .unwrap(),
                                        )
                                    } else {
                                        future::ok(ApiService::build_http_400_response())
                                    }
                                })
                                .map_err(Into::into);

                                return Box::new(result);
                            }
                        } else if let (Some(cluster_id), Some(new_cluster_id)) = (
                            hash_query.get("cluster_id"),
                            hash_query.get("new_cluster_id"),
                        ) {
                            let result =
                                db::DB::update_cluster_id(&self.db, &cluster_id, &new_cluster_id)
                                    .and_then(move |data_source| {
                                        if data_source != "No entry found" {
                                            let value = format!(
                                                "http://{}/api/cluster?data_source={}",
                                                &self.docker_host_addr, data_source,
                                            );
                                            let etcd_key = format!("clusters_{}", data_source);
                                            let data = format!(
                                                "{{\"key\": \"{}\", \"value\": \"{}\"}}",
                                                base64::encode(&etcd_key),
                                                base64::encode(&value)
                                            );
                                            let client = reqwest::Client::new();
                                            if let Err(e) =
                                                client.post(&self.etcd_url).body(data).send()
                                            {
                                                eprintln!(
                                                    "An error occurs while updating etcd: {}",
                                                    e
                                                );
                                            }
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from("Database has been updated"))
                                                    .unwrap(),
                                            )
                                        } else {
                                            future::ok(ApiService::build_http_response(
                                                StatusCode::BAD_REQUEST,
                                                "cluster_id does not exist in database",
                                            ))
                                        }
                                    })
                                    .map_err(Into::into);

                            return Box::new(result);
                        }
                    } else if hash_query.len() == 3 {
                        if let (Some(cluster_id), Some(data_source), Some(new_category_id)) = (
                            hash_query.get("cluster_id"),
                            hash_query.get("data_source"),
                            hash_query.get("new_category_id"),
                        ) {
                            if let Ok(new_category_id) = new_category_id.parse::<i32>() {
                                let resp = db::DB::update_category_id_in_clusters(&self.db, &cluster_id, &data_source, new_category_id)
                                    .then(move |update_result| match update_result {
                                        Ok(_) => future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from("The category_id has been updated"))
                                                .unwrap(),
                                        ),
                                        Err(err) => {
                                            if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                                err.kind()
                                            {
                                                if *reason == db::error::DatabaseError::DatabaseLocked {
                                                    return future::ok(
                                                        ApiService::build_http_response(StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable")
                                                    );
                                                } else if *reason
                                                    == db::error::DatabaseError::RecordNotExist
                                                {
                                                    return future::ok(
                                                        ApiService::build_http_response(StatusCode::BAD_REQUEST, "The specified record does not exist in database")
                                                    );
                                                }
                                            }
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                    });
                                return Box::new(resp);
                            }
                        } else if let (Some(cluster_id), Some(data_source), Some(new_category)) = (
                            hash_query.get("cluster_id"),
                            hash_query.get("data_source"),
                            hash_query.get("new_category"),
                        ) {
                            let resp = db::DB::update_category_in_clusters(&self.db, &cluster_id, &data_source, new_category)
                                .then(move |update_result| match update_result {
                                    Ok(_) => future::ok(
                                        Response::builder()
                                            .status(StatusCode::OK)
                                            .body(Body::from("The category has been updated"))
                                            .unwrap(),
                                    ),
                                    Err(err) => {
                                        if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                            err.kind()
                                        {
                                            if *reason == db::error::DatabaseError::DatabaseLocked {
                                                return future::ok(
                                                    ApiService::build_http_response(StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable")
                                                );
                                            } else if *reason
                                                == db::error::DatabaseError::RecordNotExist
                                            {
                                                return future::ok(
                                                    ApiService::build_http_response(StatusCode::BAD_REQUEST, "The specified record does not exist in database")
                                                );
                                            }
                                            }
                                            future::ok(ApiService::build_http_500_response())
                                        }
                                });
                            return Box::new(resp);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                (&Method::PUT, "/api/suspicious_tokens") => {
                    let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                        .into_owned()
                        .collect();
                    if hash_query.len() == 1 {
                        if let Some(etcd_key) = hash_query.get("etcd_key") {
                            let etcd_key_cloned = etcd_key.clone();
                            let result = req.into_body().concat2().map_err(Into::into).and_then(
                                move |buf| {
                                    let data = format!(
                                        "{{\"key\": \"{}\", \"value\": \"{}\"}}",
                                        base64::encode(&etcd_key_cloned),
                                        base64::encode(&buf)
                                    );
                                    let client = reqwest::Client::new();
                                    match client.post(&self.etcd_url).body(data).send() {
                                        Ok(_) => {
                                            let msg =
                                                format!("{} has been updated.", etcd_key_cloned);
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from(msg))
                                                    .unwrap(),
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
                                },
                            );
                            return Box::new(result);
                        }
                    }
                    Box::new(future::ok(ApiService::build_http_400_response()))
                }
                _ => {
                    if req.method() == Method::PUT && req.uri().path().contains("/api/cluster/") {
                        let path: Vec<&str> = req.uri().path().split('/').collect();
                        let hash_query: HashMap<_, _> = url::form_urlencoded::parse(query.as_ref())
                            .into_owned()
                            .collect();
                        if path.len() == 4
                            && hash_query.len() == 2
                            && path[1] == "api"
                            && path[2] == "cluster"
                        {
                            let cluster_id = percent_decode(path[3].as_bytes()).decode_utf8();
                            if let (Ok(cluster_id), Some(detector_id), Some(data_source)) = (
                                cluster_id,
                                hash_query.get("detector_id"),
                                hash_query.get("data_source"),
                            ) {
                                #[derive(Debug, Deserialize)]
                                struct NewValues {
                                    cluster_id: Option<String>,
                                    category: Option<String>,
                                    qualifier: Option<String>,
                                }
                                let cluster_id_cloned = cluster_id.into_owned();
                                let data_source_cloned = data_source.clone();
                                let detector_id_cloned = detector_id.clone();
                                let result = req
                                    .into_body()
                                    .concat2()
                                    .map_err(Into::into)
                                    .and_then(|buf| {
                                        serde_json::from_slice(&buf)
                                            .map(move |data: NewValues| {
                                                if data.cluster_id.is_some() || data.category.is_some() || data.qualifier.is_some() {
                                                    let mut set_query = String::new();
                                                    if let Some(new_cluster_id) = data.cluster_id {
                                                        set_query.push_str(&format!("SET cluster_id = '{}'", new_cluster_id));
                                                    }
                                                    if let Some(new_category) = data.category {
                                                        if set_query.is_empty() {
                                                            set_query.push_str(&format!("SET category_id = (SELECT category_id FROM category WHERE category = '{}')", new_category));
                                                        }
                                                        else {
                                                            set_query.push_str(&format!(", category_id = (SELECT category_id FROM category WHERE category = '{}')", new_category));
                                                        }
                                                    }
                                                    if let Some(new_qualifier) = data.qualifier {
                                                        if set_query.is_empty() {
                                                            set_query.push_str(&format!("SET qualifier_id = (SELECT qualifier_id FROM qualifier WHERE qualifier = '{}')", new_qualifier));
                                                        }
                                                        else {
                                                            set_query.push_str(&format!(", qualifier_id = (SELECT qualifier_id FROM qualifier WHERE qualifier = '{}')", new_qualifier));
                                                        }
                                                        if new_qualifier == "benign" {
                                                            let value = format!(
                                                                "http://{}/api/cluster/search?filter={{\"qualifier\": [\"benign\"], \"data_source\":[\"{}\"], \"detector_id\": [{}]}}",
                                                                &self.docker_host_addr, &data_source_cloned, &detector_id_cloned
                                                            );
                                                            ApiService::update_etcd(&self.etcd_url, &self.etcd_key, &value);
                                                        }
                                                    }
                                                    let now = chrono::Utc::now();
                                                    let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
                                                    let query = format!("UPDATE clusters {} , last_modification_time = '{}' WHERE cluster_id = '{}' and detector_id = '{}' and data_source = '{}';", set_query, timestamp, cluster_id_cloned, detector_id_cloned, data_source_cloned);
                                                    db::DB::execute_query(&self.db, &query)
                                                }
                                                else {
                                                    future::result(Err(db::error::Error::from(db::error::ErrorKind::DatabaseTransactionError(
                                                                        db::error::DatabaseError::RecordNotExist,
                                                                    ))))
                                                }
                                            })
                                            .map_err(Into::into)
                                    })
                                    .and_then(|mut result| match result.poll() {
                                        Ok(_) => future::ok(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Body::from("Cluster has been successfully updated"))
                                                .unwrap(),
                                        ),
                                        Err(e) => {
                                            if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                                e.kind()
                                            {
                                                if *reason == db::error::DatabaseError::DatabaseLocked {
                                                    future::ok(
                                                        ApiService::build_http_response(StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable")
                                                    )
                                                } else if *reason == db::error::DatabaseError::RecordNotExist {
                                                    future::ok(ApiService::build_http_response(
                                                        StatusCode::BAD_REQUEST,
                                                        "The specified record does not exist in database",
                                                    ))
                                                } else {
                                                    future::ok(ApiService::build_http_500_response())
                                                }
                                            } else {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
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
                        .and_then(|_| {
                            future::ok(
                                Response::builder()
                                    .status(StatusCode::CREATED)
                                    .body(Body::from(
                                        "New clusters have been inserted into database",
                                    ))
                                    .unwrap(),
                            )
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
                                    .unwrap(),
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
                                    .unwrap(),
                            ),
                            Err(e) => {
                                if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                    e.kind()
                                {
                                    if *reason == db::error::DatabaseError::DatabaseLocked {
                                        future::ok(ApiService::build_http_response(
                                            StatusCode::SERVICE_UNAVAILABLE,
                                            "Service temporarily unavailable",
                                        ))
                                    } else {
                                        future::ok(ApiService::build_http_500_response())
                                    }
                                } else {
                                    future::ok(ApiService::build_http_500_response())
                                }
                            }
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
                                    .body(Body::from("Cluster information has been updated"))
                                    .unwrap(),
                            ),
                            Err(e) => {
                                if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                    e.kind()
                                {
                                    if *reason == db::error::DatabaseError::DatabaseLocked {
                                        future::ok(ApiService::build_http_response(
                                            StatusCode::SERVICE_UNAVAILABLE,
                                            "Service temporarily unavailable",
                                        ))
                                    } else {
                                        future::ok(ApiService::build_http_500_response())
                                    }
                                } else {
                                    future::ok(ApiService::build_http_500_response())
                                }
                            }
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
                    let result = db::DB::get_cluster_table(&self.db)
                        .and_then(|data| future::ok(ApiService::process_clusters(data)))
                        .map_err(Into::into);
                    Box::new(result)
                }

                (&Method::GET, "/api/cluster/example") => {
                    let query = "SELECT cluster_id, examples FROM Clusters;";
                    let result = db::DB::get_cluster_examples(&self.db, query)
                        .and_then(|data| future::ok(ApiService::process_cluster_examples(data)))
                        .map_err(Into::into);
                    Box::new(result)
                }

                (&Method::PUT, "/api/cluster/qualifier") => {
                    #[derive(Debug, Deserialize)]
                    struct NewQualifier {
                        cluster_id: String,
                        data_source: String,
                        detector_id: u32,
                        qualifier: String,
                    }
                    let result = req
                        .into_body()
                        .concat2()
                        .map_err(Into::into)
                        .and_then(move |buf| {
                            match serde_json::from_slice(&buf) as Result<Vec<NewQualifier>, serde_json::error::Error> {
                                Ok(data) => {
                                    let now = chrono::Utc::now();
                                    let timestamp = chrono::NaiveDateTime::from_timestamp(now.timestamp(), 0);
                                    let mut query = String::new();
                                    data.iter().for_each(|d| query.push_str(&format!("UPDATE clusters SET qualifier_id = (SELECT qualifier_id FROM qualifier WHERE qualifier = '{}'), last_modification_time = '{}' WHERE cluster_id = '{}' and detector_id = '{}' and data_source = '{}';", d.qualifier, timestamp, d.cluster_id, d.detector_id, d.data_source)));
                                    let mut result = db::DB::execute_query(&self.db, &query);

                                    match result.poll() {
                                        Ok(_) => {
                                            data.iter().for_each(|d| {
                                                if d.qualifier == "benign" {
                                                    let value = format!(
                                                        "http://{}/api/cluster/search?filter={{\"qualifier\": [\"benign\"], \"data_source\":[\"{}\"], \"detector_id\": [{}]}}",
                                                        &self.docker_host_addr, &d.data_source, &d.detector_id
                                                    );
                                                    ApiService::update_etcd(&self.etcd_url, &self.etcd_key, &value);
                                                }
                                            });
                                            future::ok(
                                                Response::builder()
                                                    .status(StatusCode::OK)
                                                    .body(Body::from("Qualifier has been successfully updated"))
                                                    .unwrap(),
                                            )
                                        }
                                        Err(e) => {
                                            if let db::error::ErrorKind::DatabaseTransactionError(reason) =
                                                e.kind()
                                            {
                                                if *reason == db::error::DatabaseError::DatabaseLocked {
                                                    future::ok(
                                                        ApiService::build_http_response(StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable")
                                                    )
                                                } else {
                                                    future::ok(ApiService::build_http_500_response())
                                                }
                                            } else {
                                                future::ok(ApiService::build_http_500_response())
                                            }
                                        }
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
            .unwrap()
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
            .unwrap()
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
            .unwrap()
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
            .unwrap()
    }

    fn bytes_to_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| char::from(*b)).collect()
    }

    fn process_clusters(
        data: Vec<(
            db::models::ClustersTable,
            db::models::StatusTable,
            db::models::QualifierTable,
            db::models::CategoryTable,
        )>,
    ) -> Response<Body> {
        #[derive(Debug, Serialize)]
        struct Clusters {
            cluster_id: Option<String>,
            detector_id: i32,
            qualifier: String,
            status: String,
            category: String,
            signature: String,
            data_source: String,
            size: usize,
            examples: Option<Vec<db::models::Example>>,
            last_modification_time: Option<chrono::NaiveDateTime>,
        }
        let mut clusters: Vec<Clusters> = Vec::new();
        for d in data {
            let eg = d.0.examples.and_then(|eg| {
                (rmp_serde::decode::from_slice(&eg)
                    as Result<Vec<db::models::Example>, rmp_serde::decode::Error>)
                    .ok()
            });
            let size = d.0.size.parse::<usize>().unwrap_or(0);
            clusters.push(Clusters {
                cluster_id: d.0.cluster_id,
                detector_id: d.0.detector_id,
                qualifier: d.2.qualifier,
                status: d.1.status,
                category: d.3.category,
                signature: d.0.signature,
                data_source: d.0.data_source,
                size,
                examples: eg,
                last_modification_time: d.0.last_modification_time,
            });
        }
        match serde_json::to_string(&clusters) {
            Ok(json) => Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap(),
            Err(_) => ApiService::build_http_500_response(),
        }
    }

    fn process_cluster_examples(data: Vec<db::models::ClusterExample>) -> Response<Body> {
        #[derive(Debug, Serialize)]
        struct Clusters {
            cluster_id: Option<String>,
            examples: Option<Vec<db::models::Example>>,
        }
        let mut clusters: Vec<Clusters> = Vec::new();
        for d in data {
            let eg = d.examples.and_then(|eg| {
                (rmp_serde::decode::from_slice(&eg)
                    as Result<Vec<db::models::Example>, rmp_serde::decode::Error>)
                    .ok()
            });
            clusters.push(Clusters {
                cluster_id: d.cluster_id,
                examples: eg,
            });
        }
        match serde_json::to_string(&clusters) {
            Ok(json) => Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap(),
            Err(_) => ApiService::build_http_500_response(),
        }
    }

    fn update_etcd(url: &str, key: &str, value: &str) {
        let data = format!(
            "{{\"key\": \"{}\", \"value\": \"{}\"}}",
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
        if let Some(cluster_id) = &self.cluster_id {
            let cluster_id = cluster_id
                .iter()
                .map(|c| format!("cluster_id='{}'", c))
                .collect::<Vec<String>>();
            Filter::build_where_clause(&mut query, &cluster_id);
        }
        if let Some(data_source) = &self.data_source {
            let data_source = data_source
                .iter()
                .map(|d| format!("data_source='{}'", d))
                .collect::<Vec<String>>();
            Filter::build_where_clause(&mut query, &data_source);
        }
        if let Some(detector_id) = &self.detector_id {
            let detector_id = detector_id
                .iter()
                .map(|d| format!("detector_id='{}'", d))
                .collect::<Vec<String>>();
            Filter::build_where_clause(&mut query, &detector_id);
        }
        if let Some(status) = &self.status {
            let status = status
                .iter()
                .map(|s| {
                    format!(
                        "Clusters.status_id = (SELECT status_id FROM status WHERE status = '{}')",
                        s
                    )
                })
                .collect::<Vec<String>>();
            Filter::build_where_clause(&mut query, &status);
        }
        if let Some(qualifier) = &self.qualifier {
            let qualifier = qualifier.iter().map(|q| format!("Clusters.qualifier_id = (SELECT qualifier_id FROM qualifier WHERE qualifier = '{}')", q)).collect::<Vec<String>>();
            Filter::build_where_clause(&mut query, &qualifier);
        }

        query
    }

    fn build_where_clause(query: &mut Vec<String>, new_filters: &[String]) {
        if query.is_empty() {
            query.extend(new_filters.iter().map(std::string::ToString::to_string));
        } else {
            let mut new_query = Vec::<String>::new();
            while let Some(q) = query.pop() {
                new_query.extend(
                    new_filters
                        .iter()
                        .map(|f| format!("{} and {}", q, f))
                        .collect::<Vec<String>>(),
                );
            }
            query.extend(new_query);
        }
    }
}
