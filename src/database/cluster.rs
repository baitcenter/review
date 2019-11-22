use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{NaiveDateTime, Utc};
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::schema::cluster;
use crate::database::*;
use crate::server::EtcdServer;

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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct QualifierUpdate {
    cluster_id: String,
    data_source: String,
    qualifier: String,
}

pub(crate) fn get_clusters(
    pool: Data<Pool>,
    query: Query<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let default_per_page = 10;
    let max_per_page = 100;
    let cluster_schema = "(((((cluster INNER JOIN status ON cluster.status_id = status.id) \
                          INNER JOIN qualifier ON cluster.qualifier_id = qualifier.id) \
                          INNER JOIN category ON cluster.category_id = category.id) \
                          INNER JOIN data_source ON cluster.data_source_id = data_source.id) \
                          INNER JOIN raw_event ON cluster.raw_event_id = raw_event.id)";
    let select = query
        .get("select")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<HashMap<String, bool>>(s).ok())
        .map(|s| {
            s.iter()
                .filter(|s| *s.1)
                .filter_map(|s| match s.0.to_lowercase().as_str() {
                    "cluster_id" => Some("cluster.cluster_id"),
                    "detector_id" => Some("cluster.detector_id"),
                    "qualifier" => Some("qualifier.description as qualifier"),
                    "status" => Some("status.description as status"),
                    "category" => Some("category.name as category"),
                    "signature" => Some("cluster.signature"),
                    "data_source" => Some("data_source.topic_name as data_source"),
                    "size" => Some("cluster.size"),
                    "score" => Some("cluster.score"),
                    "raw_event" => Some("raw_event.data as raw_event"),
                    "event_ids" => Some("cluster.event_ids"),
                    "last_modification_time" => Some("cluster.last_modification_time"),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            vec![
                "cluster.cluster_id",
                "cluster.detector_id",
                "qualifier.description as qualifier",
                "status.description as status",
                "category.name as category",
                "cluster.signature",
                "data_source.topic_name as data_source",
                "cluster.size",
                "cluster.score",
                "cluster.last_modification_time",
                "raw_event.data as raw_event",
                "cluster.event_ids",
            ]
        });
    let where_clause = query
        .get("filter")
        .and_then(Value::as_str)
        .and_then(|f| Filter::get_where_clause(&f).ok())
        .filter(|f| !f.is_empty());
    let page = GetQuery::get_page(&query);
    let per_page = GetQuery::get_per_page(&query, max_per_page).unwrap_or_else(|| default_per_page);
    let orderby = query
        .get("orderby")
        .and_then(Value::as_str)
        .and_then(|column_name| match column_name.to_lowercase().as_str() {
            "cluster_id" => Some("cluster.cluster_id"),
            "detector_id" => Some("cluster.detector_id"),
            "qualifier" => Some("qualifier.description"),
            "status" => Some("status.description"),
            "category" => Some("category.name"),
            "signature" => Some("cluster.signature"),
            "data_source" => Some("data_source.topic_name"),
            "size" => Some("cluster.size"),
            "score" => Some("cluster.score"),
            "raw_event" => Some("raw_event.data"),
            "event_ids" => Some("cluster.event_ids"),
            "last_modification_time" => Some("cluster.last_modification_time"),
            _ => None,
        });
    let order = if orderby.is_some() {
        GetQuery::get_order(&query)
    } else {
        None
    };

    let query_result: Result<Vec<GetQueryData>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            GetQuery::new(
                select,
                cluster_schema,
                where_clause,
                page,
                per_page,
                orderby,
                order,
            )
            .get_results::<GetQueryData>(&conn)
            .map_err(Into::into)
        });

    future::result(GetQuery::build_response(&query, per_page, query_result))
}

pub(crate) fn update_cluster(
    pool: Data<Pool>,
    cluster_id: Path<String>,
    query: Query<Value>,
    new_cluster: Json<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let data_source = query.get("data_source").and_then(Value::as_str);
    let new_cluster = new_cluster.into_inner();
    let (new_cluster_id, new_category, new_qualifier) = (
        new_cluster.get("cluster_id").and_then(Value::as_str),
        new_cluster.get("category").and_then(Value::as_str),
        new_cluster.get("qualifier").and_then(Value::as_str),
    );

    if let (Some(data_source), Some(_), _, _)
    | (Some(data_source), _, Some(_), _)
    | (Some(data_source), _, _, Some(_)) =
        (data_source, new_cluster_id, new_category, new_qualifier)
    {
        let query_result: Result<_, Error> = pool.get().map_err(Into::into).and_then(|conn| {
            let cluster_id = cluster_id.into_inner();
            diesel::select(attempt_cluster_update(
                cluster_id,
                data_source,
                new_category,
                new_cluster_id,
                new_qualifier,
            ))
            .get_result::<i32>(&conn)
            .map_err(Into::into)
        });

        let result = match query_result {
            Ok(1) => Ok(HttpResponse::Ok().into()),
            Ok(_) => Ok(HttpResponse::InternalServerError().into()),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        };
        future::result(result)
    } else {
        future::result(Ok(HttpResponse::BadRequest().into()))
    }
}

pub(crate) fn update_clusters(
    pool: Data<Pool>,
    cluster_update: Json<Vec<ClusterUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl;

    #[derive(Debug, Queryable, Serialize)]
    struct Cluster {
        cluster_id: Option<String>,
        signature: String,
        event_ids: Option<Vec<BigDecimal>>,
        raw_event_id: i32,
        size: BigDecimal,
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
            ))
            .load::<Cluster>(&conn)
            .map_err(Into::into)
            .and_then(|cluster_list| {
                let replace_clusters: Vec<_> =
                    cluster_update
                        .iter()
                        .filter_map(|c| {
                            use std::str::FromStr;
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
                                let cluster_size =
                                    c.size.and_then(FromPrimitive::from_usize).map_or_else(
                                        || cluster.size.clone(),
                                        |new_size: BigDecimal| {
                                            // Reset the value of size if it exceeds 20 digits
                                            BigDecimal::from_str("100000000000000000000")
                                                .ok()
                                                .map_or(new_size.clone(), |max_size| {
                                                    if &new_size + &cluster.size < max_size {
                                                        &new_size + &cluster.size
                                                    } else {
                                                        new_size
                                                    }
                                                })
                                        },
                                    );
                                let data_source_id =
                                    get_data_source_id(&pool, &c.data_source).unwrap_or_default();
                                if data_source_id == 0 {
                                    None
                                } else {
                                    Some((
                                        dsl::cluster_id.eq(c.cluster_id.clone()),
                                        dsl::detector_id.eq(c.detector_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(cluster.raw_event_id),
                                        dsl::signature.eq(sig),
                                        dsl::size.eq(cluster_size),
                                        dsl::score.eq(c.score),
                                        dsl::data_source_id.eq(data_source_id),
                                        dsl::last_modification_time.eq(Some(timestamp)),
                                    ))
                                }
                            } else {
                                let event_ids = c.event_ids.as_ref().map(|e| {
                                    e.iter()
                                        .filter_map(|event_id| FromPrimitive::from_u64(*event_id))
                                        .collect::<Vec<BigDecimal>>()
                                });
                                let sig = match &c.signature {
                                    Some(sig) => sig.clone(),
                                    None => "-".to_string(),
                                };
                                let cluster_size: BigDecimal =
                                    c.size.and_then(FromPrimitive::from_usize).unwrap_or_else(
                                        || FromPrimitive::from_usize(1).unwrap_or_default(),
                                    );
                                let data_source_id = get_data_source_id(&pool, &c.data_source)
                                    .unwrap_or_else(|_| {
                                        add_data_source(&pool, &c.data_source, &c.data_source_type)
                                    });
                                let raw_event_id = get_empty_raw_event_id(&pool, data_source_id)
                                    .unwrap_or_default();
                                if data_source_id == 0 || raw_event_id == 0 {
                                    None
                                } else {
                                    Some((
                                        dsl::cluster_id.eq(c.cluster_id.clone()),
                                        dsl::detector_id.eq(c.detector_id),
                                        dsl::event_ids.eq(event_ids),
                                        dsl::raw_event_id.eq(raw_event_id),
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
                            dsl::event_ids.eq(excluded(dsl::event_ids)),
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

fn merge_cluster_examples(
    current_examples: Option<Vec<BigDecimal>>,
    new_examples: Option<Vec<u64>>,
) -> Option<Vec<BigDecimal>> {
    let max_example_num: usize = 25;
    new_examples.map_or(current_examples.clone(), |new_examples| {
        let new_examples = new_examples
            .into_iter()
            .filter_map(FromPrimitive::from_u64)
            .collect::<Vec<_>>();
        let mut current_eg = current_examples.unwrap_or_default();
        current_eg.extend(new_examples);
        if current_eg.len() > max_example_num {
            current_eg.sort();
            let (_, current_eg) = current_eg.split_at(current_eg.len() - max_example_num);
            Some(current_eg.to_vec())
        } else {
            Some(current_eg)
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
        .and_then(reqwest::r#async::Response::error_for_status)
        .then(move |response| tx.send(response))
        .map(|_| ())
        .map_err(|_| ());
    if let Ok(mut runtime) = tokio::runtime::Runtime::new() {
        runtime.spawn(req);
        let _ = rx.wait();
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