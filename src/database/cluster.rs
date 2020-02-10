use actix_web::{
    http,
    web::{Data, Json, Path, Payload, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::database::*;

pub(crate) async fn get_clusters(
    pool: Data<Pool>,
    query: Query<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let default_per_page = 10;
    let max_per_page = 100;
    let cluster_schema = "((((cluster INNER JOIN status ON cluster.status_id = status.id) \
                          INNER JOIN qualifier ON cluster.qualifier_id = qualifier.id) \
                          INNER JOIN category ON cluster.category_id = category.id) \
                          INNER JOIN data_source ON cluster.data_source_id = data_source.id)";
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

    GetQuery::build_response(&query, per_page, query_result)
}

pub(crate) async fn update_cluster(
    pool: Data<Pool>,
    cluster_id: Path<String>,
    query: Query<Value>,
    new_cluster: Json<Value>,
) -> Result<HttpResponse, actix_web::Error> {
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

        match query_result {
            Ok(1) => Ok(HttpResponse::Ok().into()),
            Ok(_) => Ok(HttpResponse::InternalServerError().into()),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(build_err_msg(&e))),
        }
    } else {
        Ok(HttpResponse::BadRequest().into())
    }
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

pub(crate) async fn update_clusters(
    pool: Data<Pool>,
    payload: Payload,
    max_event_id_num: Data<Mutex<usize>>,
) -> Result<HttpResponse, actix_web::Error> {
    let bytes = load_payload(payload).await?;
    let cluster_update: Vec<ClusterUpdate> = serde_json::from_slice(&bytes)?;
    let max_event_id_num: BigDecimal = match max_event_id_num.lock() {
        Ok(num) => FromPrimitive::from_usize(*num).expect("should be fine"),
        Err(e) => {
            error!(
                "Failed to acquire lock: {}. Use default max_event_id_number 25",
                e
            );
            FromPrimitive::from_usize(25).expect("should be fine")
        }
    };

    let query_result: Result<i32, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let cluster_update_clone = cluster_update.clone();
        let result = conn.transaction::<i32, Error, _>(|| {
            Ok(cluster_update_clone
                .into_iter()
                .filter_map(|c| {
                    let event_ids = c.event_ids.as_ref().map(|e| {
                        e.iter()
                            .filter_map(|event_id| FromPrimitive::from_u64(*event_id))
                            .collect::<Vec<BigDecimal>>()
                    });
                    let cluster_size: BigDecimal = c
                        .size
                        .and_then(FromPrimitive::from_usize)
                        .unwrap_or_else(|| FromPrimitive::from_usize(1).unwrap_or_default());
                    let query_result = diesel::select(attempt_cluster_upsert(
                        max_event_id_num.clone(),
                        c.cluster_id,
                        c.data_source,
                        c.data_source_type,
                        c.detector_id,
                        event_ids,
                        c.signature,
                        c.score,
                        cluster_size,
                    ))
                    .get_result::<i32>(&conn);

                    if let Err(e) = &query_result {
                        log::error!("Failed to insert/update a cluster: {}", e);
                    }
                    query_result.ok()
                })
                .sum())
        });
        update_events(&conn, &cluster_update);

        result
    });

    match query_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

fn update_events(conn: &Conn, cluster_update: &[ClusterUpdate]) {
    let mut new_events = cluster_update
        .iter()
        .filter_map(|cluster| {
            if let Ok(data_source_id) = get_data_source_id(conn, &cluster.data_source) {
                cluster
                    .event_ids
                    .as_ref()
                    .map(|event_ids| build_events(event_ids, data_source_id))
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    if !new_events.is_empty() {
        new_events.sort();
        new_events.dedup();
        if let Err(e) = add_events(&conn, &new_events) {
            log::error!("An error occurs while inserting events: {}", e);
        }
    }
}

pub(crate) async fn update_qualifiers(
    pool: Data<Pool>,
    payload: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let bytes = load_payload(payload).await?;
    let qualifier_updates: Vec<Value> = serde_json::from_slice(&bytes)?;
    let query_result: Result<i32, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let result: i32 = qualifier_updates
            .iter()
            .filter_map(|q| {
                let cluster_id = q.get("cluster_id").and_then(Value::as_str);
                let data_source = q.get("data_source").and_then(Value::as_str);
                let qualifier = q.get("qualifier").and_then(Value::as_str);

                if let (Some(cluster_id), Some(data_source), Some(qualifier)) =
                    (cluster_id, data_source, qualifier)
                {
                    diesel::select(attempt_qualifier_id_update(
                        cluster_id,
                        data_source,
                        qualifier,
                    ))
                    .get_result::<i32>(&conn)
                    .ok()
                } else {
                    None
                }
            })
            .sum();

        Ok(result)
    });

    match query_result {
        Ok(0) => Ok(HttpResponse::BadRequest().into()),
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
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
