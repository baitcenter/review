use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use chrono::{NaiveDateTime, Utc};
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::category::CategoryTable;
use super::data_source::DataSourceTable;
use super::qualifier::QualifierTable;
use super::schema;
use super::schema::cluster;
use super::status::StatusTable;

use crate::database::*;
use crate::server::EtcdServer;

type ClusterResponse = (
    Option<String>,        // cluster_id
    Option<i32>,           // detector_id
    Option<String>,        // qualifier
    Option<String>,        // status
    Option<String>,        // category
    Option<String>,        // signature
    Option<String>,        // data_source
    Option<usize>,         // size
    Option<f64>,           // score
    Option<Example>,       // examples
    Option<NaiveDateTime>, // last_modification_time
);

#[derive(
    Debug,
    AsChangeset,
    Associations,
    Identifiable,
    Insertable,
    Queryable,
    QueryableByName,
    Serialize,
)]
#[table_name = "cluster"]
#[belongs_to(CategoryTable, foreign_key = "category_id")]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
#[belongs_to(StatusTable, foreign_key = "status_id")]
#[belongs_to(QualifierTable, foreign_key = "qualifier_id")]
pub(crate) struct ClustersTable {
    id: i32,
    cluster_id: Option<String>,
    category_id: i32,
    detector_id: i32,
    event_ids: Option<Vec<u8>>,
    raw_event_id: Option<i32>,
    qualifier_id: i32,
    status_id: i32,
    signature: String,
    size: String,
    score: Option<f64>,
    data_source_id: i32,
    last_modification_time: Option<chrono::NaiveDateTime>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ClusterUpdate {
    cluster_id: String,
    detector_id: i32,
    signature: Option<String>,
    score: Option<f64>,
    data_source: String,
    data_source_type: String,
    size: Option<usize>,
    event_ids: Option<Vec<u64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Example {
    pub raw_event: String,
    pub event_ids: Vec<u64>,
}

#[derive(Debug, Deserialize)]
pub struct NewClusterValues {
    cluster_id: Option<String>,
    category: Option<String>,
    qualifier: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QualifierUpdate {
    pub cluster_id: String,
    pub data_source: String,
    pub qualifier: String,
}

pub(crate) fn add_clusters(
    pool: Data<Pool>,
    new_clusters: Json<Vec<ClusterUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl;

    let insert_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let insert_clusters: Vec<_> = new_clusters
            .into_inner()
            .iter()
            .filter_map(|c| {
                let data_source_id =
                    get_data_source_id(&pool, &c.data_source).unwrap_or_else(|_| {
                        add_data_source(&pool, &c.data_source, &c.data_source_type)
                    });
                if data_source_id == 0 {
                    None
                } else {
                    let event_ids = match &c.event_ids {
                        Some(eg) => rmp_serde::encode::to_vec(&eg).ok(),
                        None => None,
                    };

                    // Signature is required field in central repo database
                    // but if new cluster information does not have signature field,
                    // we use '-' as a signature
                    let sig = match &c.signature {
                        Some(sig) => sig.clone(),
                        None => "-".to_string(),
                    };
                    let cluster_size = match c.size {
                        Some(cluster_size) => cluster_size.to_string(),
                        None => "1".to_string(),
                    };

                    // We always insert 1 for category_id
                    // "unknown" for qualifier_id, and "pending review" for status_id.
                    Some((
                        dsl::cluster_id.eq(Some(c.cluster_id.to_string())),
                        dsl::category_id.eq(1),
                        dsl::detector_id.eq(c.detector_id),
                        dsl::event_ids.eq(event_ids),
                        dsl::raw_event_id.eq(Option::<i32>::None),
                        dsl::qualifier_id.eq(2),
                        dsl::status_id.eq(2),
                        dsl::signature.eq(sig),
                        dsl::size.eq(cluster_size),
                        dsl::score.eq(c.score),
                        dsl::data_source_id.eq(data_source_id),
                        dsl::last_modification_time.eq(Option::<NaiveDateTime>::None),
                    ))
                }
            })
            .collect();

        if insert_clusters.is_empty() {
            Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
        } else {
            diesel::insert_into(cluster::dsl::cluster)
                .values(&insert_clusters)
                .execute(&*conn)
                .map_err(Into::into)
        }
    });

    let result = match insert_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn get_cluster_table(
    pool: Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query_result: Result<Vec<ClusterResponse>, Error> = pool
        .get()
        .map_err(Into::into)
        .and_then(|conn| {
            cluster::dsl::cluster
                .inner_join(schema::status::dsl::status)
                .inner_join(schema::qualifier::dsl::qualifier)
                .inner_join(schema::category::dsl::category)
                .inner_join(schema::data_source::dsl::data_source)
                .load::<(
                    ClustersTable,
                    StatusTable,
                    QualifierTable,
                    CategoryTable,
                    DataSourceTable,
                )>(&conn)
                .map_err(Into::into)
        })
        .and_then(|data| {
            let clusters = data
                .into_iter()
                .map(|d| {
                    let event_ids = if let Some(event_ids) =
                        d.0.event_ids
                            .and_then(|eg| rmp_serde::decode::from_slice::<Vec<u64>>(&eg).ok())
                    {
                        let raw_event = if let Some(raw_event_id) = d.0.raw_event_id {
                            get_raw_event_by_raw_event_id(&pool, raw_event_id)
                                .ok()
                                .map_or("-".to_string(), |raw_events| bytes_to_string(&raw_events))
                        } else {
                            "-".to_string()
                        };
                        Some(Example {
                            raw_event,
                            event_ids,
                        })
                    } else {
                        None
                    };
                    let score = d.0.score.unwrap_or_default();

                    let cluster_size = d.0.size.parse::<usize>().unwrap_or(0);
                    (
                        d.0.cluster_id,
                        Some(d.0.detector_id),
                        Some(d.2.description),
                        Some(d.1.description),
                        Some(d.3.name),
                        Some(d.0.signature),
                        Some(d.4.topic_name),
                        Some(cluster_size),
                        Some(score),
                        event_ids,
                        d.0.last_modification_time,
                    )
                })
                .collect();
            Ok(clusters)
        });

    let result = match query_result {
        Ok(clusters) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_cluster_response(&clusters, true))),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn update_cluster(
    pool: Data<Pool>,
    cluster_id: Path<String>,
    data_source: Query<DataSourceQuery>,
    new_cluster: Json<NewClusterValues>,
    etcd_server: Data<EtcdServer>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl;

    let cluster_id = cluster_id.into_inner();
    let data_source = data_source.into_inner();
    let new_cluster = new_cluster.into_inner();

    let execution_result: Result<bool, Error> = if let Ok(data_source_id) =
        get_data_source_id(&pool, &data_source.data_source)
    {
        let query = diesel::update(dsl::cluster).filter(
            dsl::cluster_id
                .eq(cluster_id)
                .and(dsl::data_source_id.eq(data_source_id)),
        );
        let timestamp = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
        let category_id = new_cluster
            .category
            .and_then(|category| get_category_id(&pool, &category).ok());
        let (qualifier_id, is_benign) = new_cluster.qualifier.map_or((None, false), |qualifier| {
            (
                get_qualifier_id(&pool, &qualifier).ok(),
                qualifier == "benign",
            )
        });
        let status_id = match get_status_id(&pool, "reviewed") {
            Ok(id) => id,
            _ => 1,
        };
        let new_cluster_id = new_cluster.cluster_id;
        pool.get()
            .map_err(Into::into)
            .and_then(|conn| {
                if let (Some(cluster_id), Some(category_id), Some(qualifier_id)) =
                    (&new_cluster_id, &category_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::category_id.eq(category_id),
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::status_id.eq(status_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let (Some(cluster_id), Some(category_id)) =
                    (&new_cluster_id, &category_id)
                {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::category_id.eq(category_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let (Some(cluster_id), Some(qualifier_id)) =
                    (&new_cluster_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::status_id.eq(status_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let (Some(category_id), Some(qualifier_id)) =
                    (&category_id, &qualifier_id)
                {
                    query
                        .set((
                            dsl::category_id.eq(category_id),
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::status_id.eq(status_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let Some(cluster_id) = &new_cluster_id {
                    query
                        .set((
                            dsl::cluster_id.eq(cluster_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let Some(category_id) = &category_id {
                    query
                        .set((
                            dsl::category_id.eq(category_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else if let Some(qualifier_id) = &qualifier_id {
                    query
                        .set((
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::status_id.eq(status_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else {
                    Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
                }
            })
            .and_then(|_| Ok(is_benign))
    } else {
        Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
    };

    let result = match execution_result {
        Ok(is_benign) => {
            if is_benign {
                let etcd_server = &etcd_server.into_inner();
                let etcd_value = format!(
                    r#"http://{}/api/cluster/search?filter={{"qualifier": ["benign"], "data_source":["{}"]}}"#,
                    &etcd_server.docker_host_addr, &data_source.data_source
                );
                let etcd_key = format!("benign_signatures_{}", &data_source.data_source);
                update_etcd(&etcd_server.etcd_url, &etcd_key, &etcd_value);
            }
            Ok(HttpResponse::Ok().into())
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn update_clusters(
    pool: Data<Pool>,
    cluster_update: Json<Vec<ClusterUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl;

    #[derive(Debug, Queryable, Serialize)]
    pub struct Cluster {
        cluster_id: Option<String>,
        signature: String,
        event_ids: Option<Vec<u8>>,
        raw_event_id: Option<i32>,
        size: String,
        category_id: i32,
        qualifier_id: i32,
        status_id: i32,
    }
    let cluster_update = cluster_update.into_inner();
    let mut query = dsl::cluster.into_boxed();
    for cluster in &cluster_update {
        if let Ok(data_source_id) = get_data_source_id(&pool, &cluster.data_source) {
            query = query.or_filter(
                dsl::cluster_id
                    .eq(&cluster.cluster_id)
                    .and(dsl::data_source_id.eq(data_source_id)),
            );
        }
    }
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        query
            .select((
                dsl::cluster_id,
                dsl::signature,
                dsl::event_ids,
                dsl::raw_event_id,
                dsl::size,
                dsl::category_id,
                dsl::qualifier_id,
                dsl::status_id,
            ))
            .load::<Cluster>(&conn)
            .map_err(Into::into)
            .and_then(|cluster_list| {
                let replace_clusters: Vec<_> = cluster_update
                    .iter()
                    .filter_map(|c| {
                        if let Some(cluster) = cluster_list
                            .iter()
                            .find(|cluster| Some(c.cluster_id.clone()) == cluster.cluster_id)
                        {
                            c.event_ids.as_ref()?;
                            let now = Utc::now();
                            let timestamp = NaiveDateTime::from_timestamp(now.timestamp(), 0);

                            let sig = match &c.signature {
                                Some(sig) => sig.clone(),
                                None => cluster.signature.clone(),
                            };
                            let event_ids = merge_cluster_examples(
                                cluster.event_ids.clone(),
                                c.event_ids.clone(),
                            );
                            let cluster_size = match c.size {
                                Some(new_size) => {
                                    if let Ok(current_size) = cluster.size.clone().parse::<usize>()
                                    {
                                        // check if sum of new_size and current_size exceeds max_value
                                        // if it does, we cannot calculate sum anymore, so reset the value of size
                                        if new_size > usize::max_value() - current_size {
                                            new_size.to_string()
                                        } else {
                                            (current_size + new_size).to_string()
                                        }
                                    } else {
                                        cluster.size.clone()
                                    }
                                }
                                None => cluster.size.clone(),
                            };
                            let data_source_id =
                                get_data_source_id(&pool, &c.data_source).unwrap_or_default();
                            if data_source_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::cluster_id.eq(c.cluster_id.clone()),
                                    dsl::category_id.eq(cluster.category_id),
                                    dsl::detector_id.eq(c.detector_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(cluster.raw_event_id),
                                    dsl::qualifier_id.eq(cluster.qualifier_id),
                                    dsl::status_id.eq(cluster.status_id),
                                    dsl::signature.eq(sig),
                                    dsl::size.eq(cluster_size),
                                    dsl::score.eq(c.score),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::last_modification_time.eq(Some(timestamp)),
                                ))
                            }
                        } else {
                            let event_ids = match &c.event_ids {
                                Some(eg) => rmp_serde::encode::to_vec(&eg).ok(),
                                None => None,
                            };
                            let sig = match &c.signature {
                                Some(sig) => sig.clone(),
                                None => "-".to_string(),
                            };
                            let cluster_size = match c.size {
                                Some(cluster_size) => cluster_size.to_string(),
                                None => "1".to_string(),
                            };
                            let data_source_id = get_data_source_id(&pool, &c.data_source)
                                .unwrap_or_else(|_| {
                                    add_data_source(&pool, &c.data_source, &c.data_source_type)
                                });
                            if data_source_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::cluster_id.eq(c.cluster_id.clone()),
                                    dsl::category_id.eq(1),
                                    dsl::detector_id.eq(c.detector_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(None),
                                    dsl::qualifier_id.eq(2),
                                    dsl::status_id.eq(2),
                                    dsl::signature.eq(sig),
                                    dsl::size.eq(cluster_size),
                                    dsl::score.eq(c.score),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::last_modification_time.eq(None),
                                ))
                            }
                        }
                    })
                    .collect();

                if replace_clusters.is_empty() {
                    Ok(0)
                } else {
                    diesel::insert_into(dsl::cluster)
                        .values(&replace_clusters)
                        .on_conflict((dsl::cluster_id, dsl::data_source_id))
                        .do_update()
                        .set((
                            dsl::id.eq(excluded(dsl::id)),
                            dsl::category_id.eq(excluded(dsl::category_id)),
                            dsl::detector_id.eq(excluded(dsl::detector_id)),
                            dsl::event_ids.eq(excluded(dsl::event_ids)),
                            dsl::raw_event_id.eq(excluded(dsl::raw_event_id)),
                            dsl::qualifier_id.eq(excluded(dsl::qualifier_id)),
                            dsl::status_id.eq(excluded(dsl::status_id)),
                            dsl::signature.eq(excluded(dsl::signature)),
                            dsl::size.eq(excluded(dsl::size)),
                            dsl::score.eq(excluded(dsl::score)),
                            dsl::last_modification_time.eq(excluded(dsl::last_modification_time)),
                        ))
                        .execute(&*conn)
                        .map_err(Into::into)
                }
            })
    });

    let result = match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn update_qualifiers(
    pool: Data<Pool>,
    qualifier_update: Json<Vec<QualifierUpdate>>,
    etcd_server: Data<EtcdServer>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl;

    let qualifier_update = qualifier_update.into_inner();
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let timestamp = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
        let status_id = match get_status_id(&pool, "reviewed") {
            Ok(id) => id,
            _ => 1,
        };
        let row = qualifier_update
            .iter()
            .map(|q| {
                if let (Ok(qualifier_id), Ok(data_source_id)) = (
                    get_qualifier_id(&pool, &q.qualifier),
                    get_data_source_id(&pool, &q.data_source),
                ) {
                    let target = dsl::cluster.filter(
                        dsl::cluster_id
                            .eq(&q.cluster_id)
                            .and(dsl::data_source_id.eq(data_source_id)),
                    );
                    diesel::update(target)
                        .set((
                            dsl::qualifier_id.eq(qualifier_id),
                            dsl::status_id.eq(status_id),
                            dsl::last_modification_time.eq(timestamp),
                        ))
                        .execute(&conn)
                        .map_err(Into::into)
                } else {
                    Err(Error::from(ErrorKind::DatabaseTransactionError(
                        DatabaseError::RecordNotExist,
                    )))
                }
            })
            .filter_map(Result::ok)
            .collect::<Vec<usize>>()
            .iter()
            .sum();

        if row == 0 {
            Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
        } else {
            Ok(row)
        }
    });

    let etcd_server = etcd_server.into_inner();
    let result = match query_result {
        Ok(_) => {
            let update_list = qualifier_update
                .iter()
                .filter_map(|d| {
                    if d.qualifier == "benign" {
                        Some(d.data_source.clone())
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>();
            update_list.iter().for_each(|data_source| {
                let etcd_value = format!(
                    r#"http://{}/api/cluster/search?filter={{"qualifier": ["benign"], "data_source":["{}"]}}"#,
                    &etcd_server.docker_host_addr.clone(), data_source
                );
                let etcd_key = format!("benign_signatures_{}", data_source);
                update_etcd(&etcd_server.etcd_url, &etcd_key, &etcd_value);
            });
            Ok(HttpResponse::Ok().into())
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

fn merge_cluster_examples(
    current_examples: Option<Vec<u8>>,
    new_examples: Option<Vec<u64>>,
) -> Option<Vec<u8>> {
    let max_example_num: usize = 25;
    new_examples.map_or(current_examples.clone(), |new_examples| {
        if new_examples.len() >= max_example_num {
            match rmp_serde::encode::to_vec(&new_examples) {
                Ok(new_examples) => Some(new_examples),
                Err(_) => current_examples,
            }
        } else {
            current_examples.map(|current_eg| {
                match rmp_serde::decode::from_slice::<Vec<u64>>(&current_eg) {
                    Ok(mut eg) => {
                        eg.extend(new_examples);
                        let example = if eg.len() > max_example_num {
                            eg.sort();
                            let (_, eg) = eg.split_at(eg.len() - max_example_num);
                            rmp_serde::encode::to_vec(&eg)
                        } else {
                            rmp_serde::encode::to_vec(&eg)
                        };
                        example.unwrap_or(current_eg)
                    }
                    Err(_) => current_eg,
                }
            })
        }
    })
}

fn update_etcd(url: &str, key: &str, value: &str) {
    let data = format!(
        r#"{{"key": "{}", "value": "{}"}}"#,
        base64::encode(key),
        base64::encode(value)
    );
    let (tx, rx) = tokio::sync::oneshot::channel();
    let req = reqwest::r#async::Client::new()
        .post(url)
        .body(data)
        .send()
        .and_then(|response| response.error_for_status())
        .then(move |response| tx.send(response))
        .map(|_| ())
        .map_err(|_| ());
    if let Ok(mut runtime) = tokio::runtime::Runtime::new() {
        runtime.spawn(req);
        let _ = rx.wait();
    }
}

type SelectCluster = (
    bool, // cluster_id,
    bool, // detector_id
    bool, // qualifier
    bool, // status
    bool, // category
    bool, // signature
    bool, // data_source
    bool, // size
    bool, // score
    bool, // examples
    bool, // last_modification_time
);

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
                    "Cluster.category_id = (SELECT id FROM category WHERE name = '{}')",
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
                Self::build_where_clause(&query, &cluster_id)
            }
            None => query,
        };
        let query = match &self.data_source {
            Some(data_source) => {
                let data_source = data_source
                    .iter()
                    .map(|d| format!("Cluster.data_source_id = (SELECT id FROM data_source WHERE topic_name = '{}')", d))
                    .collect::<Vec<String>>();
                Self::build_where_clause(&query, &data_source)
            }
            None => query,
        };
        let query = match &self.detector_id {
            Some(detector_id) => {
                let detector_id = detector_id
                    .iter()
                    .map(|d| format!("detector_id='{}'", d))
                    .collect::<Vec<String>>();
                Self::build_where_clause(&query, &detector_id)
            }
            None => query,
        };
        let query = match &self.status {
            Some(status) => {
                let status = status
                    .iter()
                    .map(|s| {
                        format!(
                            "Cluster.status_id = (SELECT id FROM status WHERE description = '{}')",
                            s
                        )
                    })
                    .collect::<Vec<String>>();
                Self::build_where_clause(&query, &status)
            }
            None => query,
        };
        match &self.qualifier {
            Some(qualifier) => {
                let qualifier = qualifier.iter().map(|q| format!("Cluster.qualifier_id = (SELECT id FROM qualifier WHERE description = '{}')", q)).collect::<Vec<String>>();
                Self::build_where_clause(&query, &qualifier)
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
            .and_then(|filter: Self| {
                let filter = Self::query_builder(&filter);
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
    fn response_type_builder(&self) -> SelectCluster {
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

const SELECT_ALL: SelectCluster = (
    true, true, true, true, true, true, true, true, true, true, true,
);

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterSelectQuery {
    filter: Option<String>,
    limit: Option<String>,
    select: Option<String>,
}

pub(crate) fn get_selected_clusters(
    pool: Data<Pool>,
    query: Query<ClusterSelectQuery>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query = query.into_inner();
    let where_clause = query.filter.map(|f| match Filter::get_where_clause(&f) {
        Ok(where_clause) => {
            if where_clause.is_empty() {
                None
            } else {
                Some(where_clause)
            }
        }
        Err(_) => None,
    });
    let select = query
        .select
        .map_or(SELECT_ALL, |s| match serde_json::from_str::<Select>(&s) {
            Ok(s) => Select::response_type_builder(&s),
            Err(_) => SELECT_ALL,
        });
    let limit = query.limit;

    let query_result: Result<(Vec<ClusterResponse>, bool), Error> =
        pool.get()
            .map_err(Into::into)
            .and_then(|conn| {
                let mut query = "SELECT * FROM cluster INNER JOIN category ON cluster.category_id = category.id INNER JOIN qualifier ON cluster.qualifier_id = qualifier.id INNER JOIN status ON cluster.status_id = status.id INNER JOIN data_source ON cluster.data_source_id = data_source.id".to_string();
                if let Some(Some(where_clause)) = where_clause {
                    query.push_str(&format!(" WHERE {}", where_clause));
                }
                if let Some(limit) = limit {
                    query.push_str(&format!(" LIMIT {}", limit));
                }
                query.push_str(";");
                diesel::sql_query(query)
                    .load::<(
                        ClustersTable,
                        StatusTable,
                        QualifierTable,
                        CategoryTable,
                        DataSourceTable,
                    )>(&conn)
                    .map_err(Into::into)
            })
            .and_then(|data| {
                let clusters: Vec<ClusterResponse> =
                    data.into_iter()
                        .map(|d| {
                            let cluster_id = if select.0 { d.0.cluster_id } else { None };
                            let detector_id = if select.1 {
                                Some(d.0.detector_id)
                            } else {
                                None
                            };
                            let qualifier = if select.2 {
                                Some(d.2.description)
                            } else {
                                None
                            };
                            let status = if select.3 {
                                Some(d.1.description)
                            } else {
                                None
                            };
                            let category = if select.4 { Some(d.3.name) } else { None };
                            let signature = if select.5 { Some(d.0.signature) } else { None };
                            let data_source = if select.6 {
                                Some(d.4.topic_name.clone())
                            } else {
                                None
                            };
                            let cluster_size = if select.7 {
                                Some(d.0.size.parse::<usize>().unwrap_or(0))
                            } else {
                                None
                            };
                            let score = if select.8 { d.0.score } else { None };
                            let event_ids = if select.9 {
                                if let Some(event_ids) = d.0.event_ids.and_then(|eg| {
                                    rmp_serde::decode::from_slice::<Vec<u64>>(&eg).ok()
                                }) {
                                    let raw_event = if let Some(raw_event_id) = d.0.raw_event_id {
                                        get_raw_event_by_raw_event_id(&pool, raw_event_id)
                                            .ok()
                                            .map_or("-".to_string(), |raw_events| {
                                                bytes_to_string(&raw_events)
                                            })
                                    } else {
                                        "-".to_string()
                                    };
                                    Some(Example {
                                        raw_event,
                                        event_ids,
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            let time = if select.10 {
                                d.0.last_modification_time
                            } else {
                                None
                            };
                            (
                                cluster_id,
                                detector_id,
                                qualifier,
                                status,
                                category,
                                signature,
                                data_source,
                                cluster_size,
                                score,
                                event_ids,
                                time,
                            )
                        })
                        .collect();
                Ok((clusters, select.8))
            });

    let result = match query_result {
        Ok(clusters) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_cluster_response(&clusters.0, clusters.1))),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

fn build_cluster_response(data: &[ClusterResponse], return_score: bool) -> String {
    let mut json = String::new();
    json.push_str("[");
    for (index, d) in data.iter().enumerate() {
        json.push_str("{");
        let mut j = String::new();
        if let Some(cluster_id) = &d.0 {
            j.push_str(&build_response_string(
                j.is_empty(),
                "cluster_id",
                cluster_id,
            ));
        }
        if let Some(detector_id) = d.1 {
            j.push_str(&build_response_string(
                j.is_empty(),
                "detector_id",
                detector_id,
            ));
        }
        if let Some(qualifier) = &d.2 {
            j.push_str(&build_response_string(j.is_empty(), "qualifier", qualifier));
        }
        if let Some(status) = &d.3 {
            j.push_str(&build_response_string(j.is_empty(), "status", status));
        }
        if let Some(category) = &d.4 {
            j.push_str(&build_response_string(j.is_empty(), "category", category));
        }
        if let Some(signature) = &d.5 {
            j.push_str(&build_response_string(j.is_empty(), "signature", signature));
        }
        if let Some(data_source) = &d.6 {
            j.push_str(&build_response_string(
                j.is_empty(),
                "data_source",
                data_source,
            ));
        }
        if let Some(size) = &d.7 {
            j.push_str(&build_response_string(j.is_empty(), "size", size));
        }
        if return_score {
            if let Some(score) = d.8 {
                j.push_str(&build_response_string(j.is_empty(), "score", score));
            } else {
                j.push_str(&build_response_string(j.is_empty(), "score", "-"));
            }
        }
        if let Some(examples) = &d.9 {
            match serde_json::to_string(&examples) {
                Ok(e) => {
                    if j.is_empty() {
                        j.push_str(&format!(r#""examples":{}"#, e));
                    } else {
                        j.push_str(&format!(r#","examples":{}"#, e));
                    }
                }
                Err(_) => {
                    if j.is_empty() {
                        j.push_str(r#""examples":-"#);
                    } else {
                        j.push_str(r#","examples":-"#);
                    }
                }
            }
        }
        if let Some(last_modification_time) = &d.10 {
            if j.is_empty() {
                j.push_str(&format!(
                    r#""last_modification_time":"{}""#,
                    last_modification_time
                ));
            } else {
                j.push_str(&format!(
                    r#","last_modification_time":"{}""#,
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
    json
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
