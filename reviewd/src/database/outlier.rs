use actix_web::{
    http,
    web::{Data, Json, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::data_source::DataSourceTable;
use super::schema::{cluster, outlier};
use crate::database::*;

#[derive(Debug, Associations, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "outlier"]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
struct OutliersTable {
    id: i32,
    raw_event: String,
    data_source_id: i32,
    event_ids: Vec<BigDecimal>,
    raw_event_id: i32,
    size: BigDecimal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OutlierUpdate {
    outlier: Vec<u8>,
    data_source: String,
    data_source_type: String,
    event_ids: Vec<u64>,
}

pub(crate) fn add_outliers(
    pool: Data<Pool>,
    new_outliers: Json<Vec<OutlierUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use outlier::dsl;

    let insert_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let insert_outliers: Vec<_> = new_outliers
            .into_inner()
            .iter()
            .filter_map(|new_outlier| {
                let o_size: BigDecimal = FromPrimitive::from_usize(new_outlier.event_ids.len())
                    .unwrap_or_else(|| FromPrimitive::from_usize(1).unwrap_or_default());
                let event_ids = new_outlier
                    .event_ids
                    .iter()
                    .filter_map(|e| FromPrimitive::from_u64(*e))
                    .collect::<Vec<BigDecimal>>();

                get_data_source_id(&pool, &new_outlier.data_source)
                    .ok()
                    .map(|data_source_id| {
                        let raw_event_id =
                            get_empty_raw_event_id(&pool, data_source_id).unwrap_or_default();
                        (
                            dsl::raw_event.eq(bytes_to_string(&new_outlier.outlier)),
                            dsl::data_source_id.eq(data_source_id),
                            dsl::event_ids.eq(event_ids),
                            dsl::raw_event_id.eq(raw_event_id),
                            dsl::size.eq(o_size),
                        )
                    })
            })
            .collect();

        if insert_outliers.is_empty() {
            Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
        } else {
            diesel::insert_into(outlier::dsl::outlier)
                .values(&insert_outliers)
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

pub(crate) fn delete_outliers(
    pool: Data<Pool>,
    outliers: Json<Vec<String>>,
    data_source: Query<DataSourceQuery>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl as c_dsl;
    use outlier::dsl as o_dsl;

    let data_source = data_source.into_inner();
    if let (Ok(data_source_id), Ok(conn)) = (
        get_data_source_id(&pool, &data_source.data_source),
        pool.get(),
    ) {
        let outliers_from_database = o_dsl::outlier
            .filter(o_dsl::data_source_id.eq(data_source_id))
            .load::<OutliersTable>(&conn)
            .unwrap_or_default();

        let deleted_outliers = outliers
            .iter()
            .filter_map(|outlier| {
                let _ = diesel::delete(
                    o_dsl::outlier.filter(
                        o_dsl::data_source_id
                            .eq(data_source_id)
                            .and(o_dsl::raw_event.eq(outlier)),
                    ),
                )
                .execute(&conn);

                if let Some(o) = outliers_from_database
                    .iter()
                    .find(|o| o.raw_event == *outlier)
                {
                    return Some((o.event_ids.clone(), o.raw_event_id));
                }
                None
            })
            .collect::<Vec<_>>();

        let raw_event_id = get_empty_raw_event_id(&pool, data_source_id).unwrap_or_default();
        let clusters_from_database = c_dsl::cluster
            .select((c_dsl::id, c_dsl::event_ids))
            .filter(
                c_dsl::data_source_id
                    .eq(data_source_id)
                    .and(c_dsl::raw_event_id.eq(raw_event_id)),
            )
            .load::<(i32, Option<Vec<BigDecimal>>)>(&conn)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, event_ids)| {
                if let (id, Some(event_ids)) = (id, event_ids) {
                    return Some((id, event_ids));
                }
                None
            })
            .collect::<Vec<_>>();

        let update_clusters = deleted_outliers
            .into_iter()
            .filter_map(|(event_ids, raw_event_id)| {
                event_ids
                    .iter()
                    .find_map(|e| {
                        clusters_from_database
                            .iter()
                            .find_map(|(id, event_ids_from_database)| {
                                if event_ids_from_database.contains(e) {
                                    Some(id)
                                } else {
                                    None
                                }
                            })
                    })
                    .map(|id| (id, raw_event_id))
            })
            .collect::<HashMap<_, _>>();

        for (id, raw_event_id) in update_clusters {
            let _ = diesel::update(c_dsl::cluster.filter(c_dsl::id.eq(id)))
                .set(c_dsl::raw_event_id.eq(raw_event_id))
                .execute(&conn);
        }
    }

    future::result(Ok(HttpResponse::Ok().into()))
}

pub(crate) fn get_outliers(
    pool: Data<Pool>,
    query: Query<Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let default_per_page = 10;
    let max_per_page = 100;
    let outlier_schema =
        "(outlier INNER JOIN data_source ON outlier.data_source_id = data_source.id)";
    let select = vec![
        "outlier.raw_event as outlier",
        "data_source.topic_name as data_source",
        "outlier.size",
        "outlier.event_ids",
    ];
    let filter = query
        .get("filter")
        .and_then(Value::as_str)
        .and_then(|f| serde_json::from_str::<Value>(f).ok());
    let where_clause = if let Some(filter) = filter {
        filter.get("data_source")
        .and_then(Value::as_array)
        .map(|f| {
            let mut where_clause = String::new();
            for (index, f) in f.iter().enumerate() {
                if let Some(f) = f.as_str() {
                    let filter = format!("Outlier.data_source_id = (SELECT id FROM data_source WHERE topic_name = '{}')", f);
                    if index == 0 {
                        where_clause.push_str(&filter);
                    } else {
                        where_clause.push_str(&format!(" or {}", filter));
                    }
                }
            }
            where_clause
        })
    } else {
        None
    };
    let page = query
        .get("page")
        .and_then(Value::as_str)
        .and_then(|p| p.parse::<i64>().ok())
        .filter(|p| *p > 0);
    let per_page = query
        .get("per_page")
        .and_then(Value::as_str)
        .and_then(|p| p.parse::<i64>().ok())
        .filter(|p| *p > 0)
        .map(|p| if p > max_per_page { max_per_page } else { p });
    let per_page = per_page.unwrap_or_else(|| default_per_page);
    let orderby = query
        .get("orderby")
        .and_then(Value::as_str)
        .and_then(|column_name| match column_name.to_lowercase().as_str() {
            "outlier" => Some("outlier.raw_event"),
            "data_source" => Some("data_source.topic_name"),
            "size" => Some("outlier.size"),
            "event_ids" => Some("outlier.event_ids"),
            _ => None,
        });
    let order = if orderby.is_some() {
        query
            .get("order")
            .and_then(Value::as_str)
            .and_then(|order| match order.to_lowercase().as_str() {
                "desc" => Some("desc"),
                _ => None,
            })
    } else {
        None
    };

    let query_result: Result<Vec<GetQueryData>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            GetQuery::new(
                select,
                outlier_schema,
                where_clause,
                page,
                per_page,
                orderby,
                order,
            )
            .get_results::<GetQueryData>(&conn)
            .map_err(Into::into)
        });
    let result = match query_result {
        Ok(data) => {
            let pagination = match (query.get("page"), query.get("per_page")) {
                (Some(_), _) | (_, Some(_)) if !data.is_empty() => {
                    let total = data[0].count;
                    let total_pages = (total as f64 / per_page as f64).ceil() as u64;
                    Some((total, total_pages))
                }
                _ => None,
            };
            let data = data.into_iter().map(|d| d.data).collect::<Vec<Value>>();

            if let Some(pagination) = pagination {
                Ok(HttpResponse::Ok()
                    .header("X-REviewd-Total", pagination.0.to_string())
                    .header("X-REviewd-TotalPages", pagination.1.to_string())
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .json(data))
            } else {
                Ok(HttpResponse::Ok()
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .json(data))
            }
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn update_outliers(
    pool: Data<Pool>,
    outlier_update: Json<Vec<OutlierUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use outlier::dsl;

    let outlier_update = outlier_update.into_inner();
    let mut query = dsl::outlier.into_boxed();
    for outlier in &outlier_update {
        if let Ok(data_source_id) = get_data_source_id(&pool, &outlier.data_source) {
            query = query.or_filter(
                dsl::raw_event
                    .eq(bytes_to_string(&outlier.outlier))
                    .and(dsl::data_source_id.eq(data_source_id)),
            );
        }
    }
    let query_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        query
            .load::<OutliersTable>(&conn)
            .map_err(Into::into)
            .and_then(|outlier_list| {
                let replace_outliers: Vec<_> = outlier_update
                    .iter()
                    .filter_map(|o| {
                        use std::str::FromStr;
                        if let Some(outlier) = outlier_list
                            .iter()
                            .find(|outlier| bytes_to_string(&o.outlier) == outlier.raw_event)
                        {
                            let o_size = FromPrimitive::from_usize(o.event_ids.len()).map_or_else(
                                || outlier.size.clone(),
                                |new_size: BigDecimal| {
                                    // Reset the value of size if it exceeds 20 digits
                                    BigDecimal::from_str("100000000000000000000").ok().map_or(
                                        new_size.clone(),
                                        |max_size| {
                                            if &new_size + &outlier.size < max_size {
                                                &new_size + &outlier.size
                                            } else {
                                                new_size
                                            }
                                        },
                                    )
                                },
                            );
                            let mut event_ids = o
                                .event_ids
                                .iter()
                                .filter_map(|e| FromPrimitive::from_u64(*e))
                                .collect::<Vec<BigDecimal>>();
                            event_ids.extend(outlier.event_ids.clone());
                            // only store most recent 25 event_ids per outlier
                            let event_ids = if event_ids.len() > 25 {
                                event_ids.sort();
                                let (_, event_ids) = event_ids.split_at(event_ids.len() - 25);
                                event_ids.to_vec()
                            } else {
                                event_ids
                            };
                            let data_source_id =
                                get_data_source_id(&pool, &o.data_source).unwrap_or_default();
                            if data_source_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::raw_event.eq(outlier.raw_event.clone()),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(outlier.raw_event_id),
                                    dsl::size.eq(o_size),
                                ))
                            }
                        } else {
                            // only store most recent 25 event_ids per outlier
                            let mut event_ids = o
                                .event_ids
                                .iter()
                                .filter_map(|e| FromPrimitive::from_u64(*e))
                                .collect::<Vec<BigDecimal>>();
                            let event_ids = if event_ids.len() > 25 {
                                event_ids.sort();
                                let (_, event_ids) = event_ids.split_at(o.event_ids.len() - 25);
                                event_ids.to_vec()
                            } else {
                                event_ids
                            };
                            let size: BigDecimal = FromPrimitive::from_usize(o.event_ids.len())
                                .unwrap_or_else(|| {
                                    FromPrimitive::from_usize(1).unwrap_or_default()
                                });
                            let data_source_id = get_data_source_id(&pool, &o.data_source)
                                .unwrap_or_else(|_| {
                                    add_data_source(&pool, &o.data_source, &o.data_source_type)
                                });
                            let raw_event_id =
                                get_empty_raw_event_id(&pool, data_source_id).unwrap_or_default();
                            if data_source_id == 0 || raw_event_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::raw_event.eq(bytes_to_string(&o.outlier)),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(raw_event_id),
                                    dsl::size.eq(size),
                                ))
                            }
                        }
                    })
                    .collect();

                if replace_outliers.is_empty() {
                    Err(ErrorKind::DatabaseTransactionError(DatabaseError::Other).into())
                } else {
                    diesel::insert_into(dsl::outlier)
                        .values(&replace_outliers)
                        .on_conflict((dsl::raw_event, dsl::data_source_id))
                        .do_update()
                        .set((
                            dsl::event_ids.eq(excluded(dsl::event_ids)),
                            dsl::raw_event_id.eq(excluded(dsl::raw_event_id)),
                            dsl::size.eq(excluded(dsl::size)),
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
