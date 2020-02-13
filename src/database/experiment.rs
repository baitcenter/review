use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use itertools::izip;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use statistical::*;
use std::collections::HashMap;

use super::schema::{cluster, column_description, data_source, top_n_datetime};
use crate::database::{self, build_err_msg};

const R_SQUARE: f64 = 0.5;
const REGRESSION_TOP_N: usize = 50;

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

#[derive(Debug, Deserialize)]
pub(crate) struct DataSourceSelectQuery {
    data_source: String,
}

#[derive(Debug, Queryable)]
struct TopNDateTimeLoadOfCluster {
    pub column_index: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<NaiveDateTime>,
    pub count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNDateTimeLoadOfDataSource {
    pub cluster_id: Option<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeSeriesLinearRegression {
    pub cluster_id: String,
    pub series: Vec<(NaiveDateTime, usize)>,
    pub slope: f64,
    pub intercept: f64,
    pub r_square: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TopNTimeSeriesTransfer {
    pub column_index: usize,
    pub top_n: Vec<TimeSeriesLinearRegression>,
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
    let query_result: Result<Vec<TopNDateTimeLoadOfCluster>, database::Error> =
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
                .load::<TopNDateTimeLoadOfCluster>(&conn)
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

pub(crate) async fn get_top_n_of_cluster_time_series_by_linear_regression(
    pool: Data<database::Pool>,
    query: Query<DataSourceSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as col_d;
    use data_source::dsl as d_d;
    use top_n_datetime::dsl as top_d;
    let query_result: Result<Vec<TopNDateTimeLoadOfDataSource>, database::Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            c_d::cluster
                .inner_join(d_d::data_source.on(c_d::data_source_id.eq(d_d::id)))
                .inner_join(col_d::column_description.on(col_d::cluster_id.eq(c_d::id)))
                .inner_join(top_d::top_n_datetime.on(top_d::description_id.eq(col_d::id)))
                .select((
                    c_d::cluster_id,
                    col_d::column_index,
                    col_d::id,
                    top_d::ranking,
                    top_d::value,
                    top_d::count,
                ))
                .filter(d_d::topic_name.eq(&query.data_source))
                .load::<TopNDateTimeLoadOfDataSource>(&conn)
                .map_err(Into::into)
        });

    match query_result {
        Ok(values) => {
            let mut series: HashMap<usize, HashMap<String, HashMap<NaiveDateTime, usize>>> =
                HashMap::new(); // TODO: BigDecimal later?

            for v in values {
                if let (Some(column_index), Some(cluster_id), Some(value), Some(count)) = (
                    v.column_index.to_usize(),
                    v.cluster_id,
                    v.value,
                    v.count.and_then(|c| c.to_usize()),
                ) {
                    *series
                        .entry(column_index)
                        .or_insert_with(HashMap::<String, HashMap<NaiveDateTime, usize>>::new)
                        .entry(cluster_id)
                        .or_insert_with(HashMap::<NaiveDateTime, usize>::new)
                        .entry(value)
                        .or_insert(0_usize) += count;
                }
            }

            let mut series: Vec<TopNTimeSeriesTransfer> = series
                .iter()
                .map(|(column_index, cluster_series)| TopNTimeSeriesTransfer {
                    column_index: *column_index,
                    top_n: cluster_series
                        .iter()
                        .map(|(cluster_id, series)| {
                            let y_values: Vec<usize> =
                                series.iter().map(|(_, count)| *count).collect();
                            let x_values: Vec<f64> = (0..y_values.len())
                                .map(|x| x.to_f64().expect("safe: usize -> f64"))
                                .collect();
                            let y_values: Vec<f64> = y_values
                                .iter()
                                .map(|y| y.to_f64().expect("safe: usize -> f64"))
                                .collect();
                            let (slope, intercept, r_square) =
                                linear_regression(&x_values, &y_values);
                            TimeSeriesLinearRegression {
                                cluster_id: cluster_id.clone(),
                                series: series.iter().map(|(dt, count)| (*dt, *count)).collect(),
                                slope,
                                intercept,
                                r_square,
                            }
                        })
                        .collect(),
                })
                .collect();

            for s in &mut series {
                s.top_n.retain(|x| x.r_square > R_SQUARE);
            }
            for s in &mut series {
                s.top_n
                    .sort_by(|a, b| b.slope.partial_cmp(&a.slope).expect("always comparable"));
            }
            for s in &mut series {
                s.top_n.truncate(REGRESSION_TOP_N);
            }

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(series))
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

#[allow(clippy::similar_names)]
fn linear_regression(x_values: &[f64], y_values: &[f64]) -> (f64, f64, f64) {
    let x_mean = mean(&x_values);
    let y_mean = mean(&y_values);
    let mut up: f64 = 0.;
    let mut down: f64 = 0.;
    for (x, y) in izip!(x_values.iter(), y_values.iter()) {
        up += (x - x_mean) * (y - y_mean);
        down += (x - x_mean).powi(2);
    }
    let slope = up / down;
    let intercept = y_mean - slope * x_mean;

    let mut rss: f64 = 0.;
    for (x, y) in izip!(x_values.iter(), y_values.iter()) {
        rss += (y - (intercept + slope * x)).powi(2);
    }

    let mut tss: f64 = 0.;
    for y in y_values.iter() {
        tss += (*y - y_mean).powi(2);
    }
    let r_square = 1. - rss / tss;

    (slope, intercept, r_square)
}
