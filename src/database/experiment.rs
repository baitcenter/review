use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use chfft::CFft1D;
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use itertools::izip;
use num_complex::Complex;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use statistical::*;
use std::collections::HashMap;

use super::schema::{cluster, column_description, data_source, top_n_datetime};
use crate::database::{self, build_err_msg};

const R_SQUARE: f64 = 0.5;
const SLOPE: f64 = 1.0;
const REGRESSION_TOP_N: usize = 50;
const MAX_HOUR_DIFF: i64 = 96;

#[derive(Debug, Deserialize)]
pub(crate) struct SizeSelectQuery {
    data_source: String,
}

#[derive(Debug, Queryable, Serialize)]
struct SizeOfClustersResponse {
    size: BigDecimal,
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
    column_index: i32,
    description_id: i32,
    ranking: Option<i64>,
    value: Option<NaiveDateTime>,
    count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNDateTimeLoadOfDataSource {
    cluster_id: Option<String>,
    column_index: i32,
    description_id: i32,
    ranking: Option<i64>,
    value: Option<NaiveDateTime>,
    count: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeSeriesTransfer {
    column_index: usize,
    series: Vec<(NaiveDateTime, usize)>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeSeriesLinearRegression {
    series: Vec<(NaiveDateTime, usize)>,
    slope: f64,
    intercept: f64,
    r_square: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeSeriesLinearRegressionOfCluster {
    cluster_id: String,
    split_series: Vec<TimeSeriesLinearRegression>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TopNTimeSeriesLinearRegressionOfCluster {
    column_index: usize,
    top_n: Vec<TimeSeriesLinearRegressionOfCluster>,
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

            let mut series: Vec<TopNTimeSeriesLinearRegressionOfCluster> = series
                .iter()
                .map(|(column_index, cluster_series)| TopNTimeSeriesLinearRegressionOfCluster {
                    column_index: *column_index,
                    top_n: cluster_series
                        .iter()
                        .map(|(cluster_id, series)| {
                            let series_vec: Vec<(NaiveDateTime, usize)> = series.iter().map(|(dt, count)| (*dt, *count)).collect();
//                            let series_vec = fill_vacant_hours(&series_vec);

                            let parts_of_series: Vec<Vec<(NaiveDateTime, usize)>> = split_series(&series_vec);
                            let cluster_time_series: Vec<TimeSeriesLinearRegression> = parts_of_series.iter().filter_map(|series| {
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

                                    if r_square > R_SQUARE && slope > SLOPE {
                                        Some(TimeSeriesLinearRegression {
                                            series: series.clone(),
                                            slope,
                                            intercept,
                                            r_square,
                                        })
                                    } else {
                                        None
                                    }
                                }).collect();
                            TimeSeriesLinearRegressionOfCluster {
                                cluster_id: cluster_id.clone(),
                                split_series: cluster_time_series,
                            }
                        }).collect(),
                   }).collect();

            for s in &mut series {
                s.top_n
                    .sort_by(|a, b| b.split_series.len().cmp(&a.split_series.len()));
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

#[must_use]
fn split_series(series: &[(NaiveDateTime, usize)]) -> Vec<Vec<(NaiveDateTime, usize)>> {
    let mut data: Vec<Complex<f64>> = Vec::new();
    for index in 1..series.len() {
        data.push(Complex::new(
            (series[index].1.to_f64().expect("safe: usize -> f64")  / series[index - 1].1.to_f64().expect("safe: usize -> f64")).ln(),
            0.0));
    }
    let mut fft = CFft1D::<f64>::with_len(data.len());
    let mut data = fft.forward(&data);

    for d in data.iter_mut() {
        d.im = 0.0;
    }
    let mut fft = CFft1D::<f64>::with_len(data.len());
    let data = fft.backward(&data);

    let mut locations: Vec<usize> = Vec::new();
    for index in 1..data.len() {
        if data[index - 1].re > 0.0 && data[index].re < 0.0 || data[index - 1].re < 0.0 && data[index].re > 0.0 {
            locations.push(index);
        }
    }

    let mut parts_of_series: Vec<Vec<(NaiveDateTime, usize)>> = Vec::new();
    let mut series = series.clone();
    for index in 0..locations.len() {
        let split_location = if index == 0 {
            locations[index]
        } else {
            locations[index] - locations[index - 1]
        };
        let (left, right) = series.split_at(split_location);
        parts_of_series.push(left.to_vec());
        series = right;
    }
    parts_of_series.push(series.to_vec());

    parts_of_series
}

#[must_use]
pub fn fill_vacant_hours(series: &[(NaiveDateTime, usize)]) -> Vec<(NaiveDateTime, usize)> {
    let mut filled_series: Vec<(NaiveDateTime, usize)> = Vec::new();
    for (index, hour) in series.iter().enumerate() {
        if index == 0 {
            filled_series.push(hour.clone());
            continue;
        } else {
            let time_diff = (hour.0 - series[index - 1].0).num_hours();
            let time_diff = if time_diff > 1 && time_diff < MAX_HOUR_DIFF {
                time_diff
            } else if time_diff >= MAX_HOUR_DIFF {
                MAX_HOUR_DIFF
            } else {
                0
            };

            if time_diff > 0 {
                for h in 1..time_diff {
                    filled_series.push((series[index - 1].0 + Duration::hours(h), 0));
                }
            }
            filled_series.push(hour.clone());
        }
    }
    filled_series
}
