use actix_web::{
    http,
    web::{Data, Json, Path, Query},
    HttpResponse,
};
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use futures::{future, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::data_source::DataSourceTable;
use super::schema::{cluster, data_source, outlier};
use crate::database::*;

#[derive(Debug, Associations, Insertable, AsChangeset, Queryable, Serialize)]
#[table_name = "outlier"]
#[belongs_to(DataSourceTable, foreign_key = "data_source_id")]
struct OutliersTable {
    id: i32,
    raw_event: Vec<u8>,
    data_source_id: i32,
    event_ids: Vec<u8>,
    raw_event_id: Option<i32>,
    size: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OutlierUpdate {
    outlier: Vec<u8>,
    data_source: String,
    data_source_type: String,
    event_ids: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct OutlierResponse {
    outlier: String,
    data_source: String,
    size: usize,
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
                let o_size = Some(new_outlier.event_ids.len().to_string());
                let event_ids =
                    rmp_serde::encode::to_vec(&new_outlier.event_ids).unwrap_or_default();
                get_data_source_id(&pool, &new_outlier.data_source)
                    .ok()
                    .map(|data_source_id| {
                        (
                            dsl::raw_event.eq(new_outlier.outlier.to_vec()),
                            dsl::data_source_id.eq(data_source_id),
                            dsl::event_ids.eq(event_ids),
                            dsl::raw_event_id.eq(Option::<i32>::None),
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
                            .and(o_dsl::raw_event.eq(outlier.as_bytes())),
                    ),
                )
                .execute(&conn);

                if let Some(o) = outliers_from_database
                    .iter()
                    .find(|o| o.raw_event == outlier.as_bytes())
                {
                    if let (event_ids, Some(raw_event_id)) = (o.event_ids.as_ref(), o.raw_event_id)
                    {
                        if let Ok(event_ids) = rmp_serde::decode::from_slice::<Vec<u64>>(event_ids)
                        {
                            return Some((event_ids, raw_event_id));
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        let clusters_from_database = c_dsl::cluster
            .select((c_dsl::id, c_dsl::event_ids))
            .filter(
                c_dsl::data_source_id
                    .eq(data_source_id)
                    .and(c_dsl::raw_event_id.is_null()),
            )
            .load::<(i32, Option<Vec<u8>>)>(&conn)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, event_ids)| {
                if let (id, Some(event_ids)) = (id, event_ids) {
                    if let Ok(event_ids) = rmp_serde::decode::from_slice::<Vec<u64>>(&event_ids) {
                        return Some((id, event_ids));
                    }
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

pub(crate) fn get_outlier_table(
    pool: Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let query_result: Result<Vec<(OutliersTable, DataSourceTable)>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            outlier::dsl::outlier
                .inner_join(data_source::dsl::data_source)
                .load::<(OutliersTable, DataSourceTable)>(&conn)
                .map_err(Into::into)
        });

    let result = match query_result {
        Ok(outliers_table) => Ok(build_http_response(outliers_table)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn get_outlier_by_data_source(
    pool: Data<Pool>,
    data_source: Path<String>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use schema::outlier::dsl;

    let data_source = data_source.into_inner();
    let query_result: Result<Vec<(OutliersTable, DataSourceTable)>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            if let Ok(data_source_id) = get_data_source_id(&pool, &data_source) {
                dsl::outlier
                    .inner_join(schema::data_source::dsl::data_source)
                    .filter(dsl::data_source_id.eq(data_source_id))
                    .load::<(OutliersTable, DataSourceTable)>(&conn)
                    .map_err(Into::into)
            } else {
                Ok(Vec::<(OutliersTable, DataSourceTable)>::new())
            }
        });

    let result = match query_result {
        Ok(outliers_table) => Ok(build_http_response(outliers_table)),
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
                    .eq(&outlier.outlier)
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
                        if let Some(outlier) = outlier_list
                            .iter()
                            .find(|outlier| o.outlier == outlier.raw_event)
                        {
                            let new_size = o.event_ids.len();
                            let o_size = match &outlier.size {
                                Some(current_size) => {
                                    if let Ok(current_size) = current_size.parse::<usize>() {
                                        // check if sum of new_size and current_size exceeds max_value
                                        // if it does, we cannot calculate sum anymore, so reset the value of size
                                        if new_size > usize::max_value() - current_size {
                                            new_size.to_string()
                                        } else {
                                            (current_size + new_size).to_string()
                                        }
                                    } else {
                                        new_size.to_string()
                                    }
                                }
                                None => new_size.to_string(),
                            };
                            let mut event_ids =
                                rmp_serde::decode::from_slice::<Vec<u64>>(&outlier.event_ids)
                                    .unwrap_or_default();
                            event_ids.extend(&o.event_ids);
                            // only store most recent 25 event_ids per outlier
                            let event_ids = if event_ids.len() > 25 {
                                event_ids.sort();
                                let (_, event_ids) = event_ids.split_at(event_ids.len() - 25);
                                event_ids
                            } else {
                                &event_ids
                            };
                            let event_ids =
                                rmp_serde::encode::to_vec(event_ids).unwrap_or_default();
                            let data_source_id =
                                get_data_source_id(&pool, &o.data_source).unwrap_or_default();
                            if data_source_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::raw_event.eq(o.outlier.clone()),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(outlier.raw_event_id),
                                    dsl::size.eq(Some(o_size)),
                                ))
                            }
                        } else {
                            // only store most recent 25 event_ids per outlier
                            let mut event_ids_cloned = o.event_ids.clone();
                            let event_ids = if event_ids_cloned.len() > 25 {
                                event_ids_cloned.sort();
                                let (_, event_ids) =
                                    event_ids_cloned.split_at(o.event_ids.len() - 25);
                                event_ids
                            } else {
                                &o.event_ids
                            };
                            let size = o.event_ids.len();
                            let event_ids =
                                rmp_serde::encode::to_vec(event_ids).unwrap_or_default();
                            let data_source_id = get_data_source_id(&pool, &o.data_source)
                                .unwrap_or_else(|_| {
                                    add_data_source(&pool, &o.data_source, &o.data_source_type)
                                });
                            if data_source_id == 0 {
                                None
                            } else {
                                Some((
                                    dsl::raw_event.eq(o.outlier.clone()),
                                    dsl::data_source_id.eq(data_source_id),
                                    dsl::event_ids.eq(event_ids),
                                    dsl::raw_event_id.eq(Option::<i32>::None),
                                    dsl::size.eq(Some(size.to_string())),
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
                            dsl::id.eq(excluded(dsl::id)),
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

fn build_http_response(data: Vec<(OutliersTable, DataSourceTable)>) -> HttpResponse {
    let resp = data
        .into_iter()
        .map(|d| {
            let event_ids =
                rmp_serde::decode::from_slice::<Vec<u64>>(&d.0.event_ids).unwrap_or_default();
            let size =
                d.0.size
                    .map_or(0, |size| size.parse::<usize>().unwrap_or(0));
            OutlierResponse {
                outlier: bytes_to_string(&d.0.raw_event),
                data_source: d.1.topic_name,
                size,
                event_ids,
            }
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .json(resp)
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}