use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{cluster, column_description, data_source, top_n_datetime};
use crate::database::{self, build_err_msg};

#[derive(Debug, Deserialize)]
pub(crate) struct SizeSelectQuery {
    data_source: String,
}

#[derive(Debug, Queryable, Serialize)]
struct SizeOfClustersResponse {
    pub size: BigDecimal,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterSelectQuery {
    data_source: String,
    cluster_id: String,
}

#[derive(Debug, Queryable)]
struct TopNDatetimeLoad {
    pub column_index: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<NaiveDateTime>,
    pub count: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeSeriesTransfer {
    pub column_index: usize,
    pub series: Vec<(NaiveDateTime, usize)>,
}

pub(crate) async fn get_sum_of_cluster_sizes(
    pool: Data<database::Pool>,
    query: Query<SizeSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use cluster::dsl as c_d;
    use data_source::dsl as d_d;
    let query_result: Result<Vec<BigDecimal>, database::Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            c_d::cluster
                .inner_join(d_d::data_source.on(c_d::data_source_id.eq(d_d::id)))
                .select(c_d::size)
                .filter(d_d::topic_name.eq(&query.data_source))
                .load::<BigDecimal>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(sizes) => {
            let mut total: BigDecimal = BigDecimal::from_usize(0).unwrap(); // TODO: Exception handling
            for q in sizes {
                total += q;
            }

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(SizeOfClustersResponse { size: total }))
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

pub(crate) async fn get_cluster_time_series(
    pool: Data<database::Pool>,
    query: Query<ClusterSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as col_d;
    use data_source::dsl as d_d;
    use top_n_datetime::dsl as top_d;
    let query_result: Result<Vec<TopNDatetimeLoad>, database::Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            c_d::cluster
                .inner_join(d_d::data_source.on(c_d::data_source_id.eq(d_d::id)))
                .inner_join(col_d::column_description.on(col_d::cluster_id.eq(c_d::id)))
                .inner_join(top_d::top_n_datetime.on(top_d::description_id.eq(col_d::id)))
                .select((
                    col_d::column_index,
                    col_d::id,
                    top_d::ranking,
                    top_d::value,
                    top_d::count,
                ))
                .filter(
                    d_d::topic_name
                        .eq(&query.data_source)
                        .and(c_d::cluster_id.eq(&query.cluster_id)),
                )
                .load::<TopNDatetimeLoad>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(values) => {
            let mut series: HashMap<usize, HashMap<NaiveDateTime, usize>> = HashMap::new(); // TODO: BigDecimal later?

            for v in values {
                if let (Some(column_index), Some(value), Some(count)) = (
                    v.column_index.to_usize(),
                    v.value,
                    v.count.and_then(|c| c.to_usize()),
                ) {
                    *series
                        .entry(column_index)
                        .or_insert_with(HashMap::<NaiveDateTime, usize>::new)
                        .entry(value)
                        .or_insert(0_usize) += count;
                }
            }

            let series: Vec<TimeSeriesTransfer> = series
                .iter()
                .map(|(column_index, top_n)| {
                    let mut series: Vec<(NaiveDateTime, usize)> =
                        top_n.iter().map(|(dt, count)| (*dt, *count)).collect();
                    series.sort_by(|a, b| a.0.cmp(&b.0));
                    TimeSeriesTransfer {
                        column_index: *column_index,
                        series,
                    }
                })
                .collect();

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(series))
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
