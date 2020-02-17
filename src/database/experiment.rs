#![allow(clippy::len_zero)]
use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use itertools::izip;
use num_traits::ToPrimitive;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FFTplanner;
use serde::{Deserialize, Serialize};
use statistical::*;
use std::collections::HashMap;

use super::schema::{cluster, column_description, data_source, top_n_datetime};
use crate::database::{self, build_err_msg};

const MIN_R_SQUARE: f64 = 0.1; // 0 <= R^2 <= 1. if R^2 = 1, y_i = linear_function(x_i) with all i.
const MIN_SLOPE: f64 = 300.0;
const REGRESSION_TOP_N: usize = 20;
const MAX_HOUR_DIFF: i64 = 96;
const TREND_SENSITIVITY: f64 = 0.35; // how much rate of the right will be 0. The smaller, the more sensible

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
    split: Option<bool>,
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
    split_series: Option<Vec<Vec<(NaiveDateTime, usize)>>>,
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
    series: Vec<(NaiveDateTime, usize)>,
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

            let mut series: Vec<TimeSeriesTransfer> = series
                .iter()
                .filter_map(|(column_index, top_n)| {
                    if top_n.len() > 0 {
                        let mut series: Vec<(NaiveDateTime, usize)> =
                            top_n.iter().map(|(dt, count)| (*dt, *count)).collect();
                        series.sort_by(|a, b| a.0.cmp(&b.0));
                        let series = fill_vacant_hours(&series, MAX_HOUR_DIFF);
                        let split_series = if let Some(true) = query.split {
                            if series.len() > 0 {
                                Some(split_series(&series))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        Some(TimeSeriesTransfer {
                            column_index: *column_index,
                            series,
                            split_series,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            series.sort_by(|a, b| a.column_index.cmp(&b.column_index));

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .json(series))
        }
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

#[allow(clippy::too_many_lines)]
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
                .map(
                    |(column_index, cluster_series)| TopNTimeSeriesLinearRegressionOfCluster {
                        column_index: *column_index,
                        top_n: cluster_series
                            .iter()
                            .filter_map(|(cluster_id, series)| {
                                if series.len() > 0 {
                                    let mut series: Vec<(NaiveDateTime, usize)> =
                                        series.iter().map(|(dt, count)| (*dt, *count)).collect();
                                    series.sort_by(|a, b| a.0.cmp(&b.0));
                                    let series = fill_vacant_hours(&series, MAX_HOUR_DIFF);
                                    let parts_of_series: Vec<Vec<(NaiveDateTime, usize)>> =
                                        split_series(&series);
                                    let cluster_time_series: Vec<TimeSeriesLinearRegression> =
                                        parts_of_series
                                            .iter()
                                            .filter_map(|series| {
                                                let y_values: Vec<usize> = series
                                                    .iter()
                                                    .map(|(_, count)| *count)
                                                    .collect();
                                                let x_values: Vec<f64> = (0..y_values.len())
                                                    .map(|x| {
                                                        x.to_f64().expect("safe: usize -> f64")
                                                    })
                                                    .collect();
                                                let y_values: Vec<f64> = y_values
                                                    .iter()
                                                    .map(|y| {
                                                        y.to_f64().expect("safe: usize -> f64")
                                                    })
                                                    .collect();
                                                let (slope, intercept, r_square) =
                                                    linear_regression(&x_values, &y_values);

                                                if r_square > MIN_R_SQUARE && slope > MIN_SLOPE {
                                                    Some(TimeSeriesLinearRegression {
                                                        series: series.clone(),
                                                        slope,
                                                        intercept,
                                                        r_square,
                                                    })
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                    if cluster_time_series.len() > 0 {
                                        Some(TimeSeriesLinearRegressionOfCluster {
                                            cluster_id: cluster_id.clone(),
                                            series,
                                            split_series: cluster_time_series,
                                        })
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    },
                )
                .collect();

            for s in &mut series {
                s.top_n
                    .sort_by(|a, b| b.split_series.len().cmp(&a.split_series.len()));
            }
            for s in &mut series {
                s.top_n.truncate(REGRESSION_TOP_N);
            }

            series.sort_by(|a, b| a.column_index.cmp(&b.column_index));

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
    if series.len() == 0 {
        return Vec::new();
    } else if series.len() == 1 {
        let mut data: Vec<Vec<(NaiveDateTime, usize)>> = Vec::new();
        data.push(series.to_vec());
        return data;
    }

    let mut input: Vec<Complex<f64>> = series
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if index == 0 {
                None
            } else {
                let up = value.1 + 1;
                let down = series[index - 1].1 + 1;
                let re: f64 = (up.to_f64().expect("safe: usize -> f64")
                    / down.to_f64().expect("safe: usize -> f64"))
                .ln();
                Some(Complex::new(re, 0.0))
            }
        })
        .collect();
    let mut output: Vec<Complex<f64>> = vec![Complex::zero(); input.len()];
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(input.len());
    fft.process(&mut input, &mut output);

    let erase_point = output.len()
        - (output.len().to_f64().expect("safe: usize -> f64") * TREND_SENSITIVITY)
            .trunc()
            .to_usize()
            .expect("safe: usize -> f64 -> usize");
    for o in output.iter_mut().skip(erase_point) {
        *o = Complex::new(0.0, 0.0);
    }

    let mut data: Vec<Complex<f64>> = vec![Complex::zero(); output.len()];
    let mut planner = FFTplanner::new(true);
    let fft = planner.plan_fft(output.len());
    fft.process(&mut output, &mut data);

    let mut locations: Vec<usize> = Vec::new();
    if data.len() > 1 {
        let mut sign: i32 = if data[0].re < 0.0 {
            -1
        } else if data[0].re == 0.0 {
            0
        } else {
            1
        };

        for index in 1..data.len() {
            let this_sign: i32 = if data[index].re < 0.0 {
                -1
            } else if data[index].re == 0.0 {
                0
            } else {
                1
            };

            if sign == 0 && this_sign != 0 {
                sign = this_sign;
            } else if sign < 0 && this_sign > 0 || sign > 0 && this_sign < 0 {
                if sign > 0 {
                    let (mut max, prev_index) = if locations.len() > 0 {
                        (
                            data[locations[locations.len() - 1] + 1].re,
                            locations[locations.len() - 1] + 1,
                        )
                    } else {
                        (data[0].re, 0)
                    };
                    let mut max_index = prev_index;
                    for (i, d) in data.iter().enumerate().take(index).skip(prev_index) {
                        if d.re > max {
                            max = d.re;
                            max_index = i;
                        }
                    }
                    locations.push(max_index + 1);
                } else {
                    locations.push(index);
                }
                sign = this_sign;
            }
        }
    }

    let mut parts_of_series: Vec<Vec<(NaiveDateTime, usize)>> = Vec::new();
    let mut series = series;
    for index in 0..locations.len() {
        let split_location = if index == 0 {
            locations[index] + 1
        } else {
            locations[index] - locations[index - 1] + 1
        };
        let (_, right) = series.split_at(split_location - 1);
        let (left, _) = series.split_at(split_location);
        let (_, left) = left.split_at(locate_next_of_front_small_ones(left));
        parts_of_series.push(left.to_vec());
        series = right;
    }
    let (_, right) = series.split_at(locate_next_of_front_small_ones(series));
    parts_of_series.push(right.to_vec());

    parts_of_series
}

#[must_use]
pub fn fill_vacant_hours(
    series: &[(NaiveDateTime, usize)],
    max_hour_diff: i64,
) -> Vec<(NaiveDateTime, usize)> {
    let mut filled_series: Vec<(NaiveDateTime, usize)> = Vec::new();
    for (index, hour) in series.iter().enumerate() {
        if index == 0 {
            filled_series.push(hour.clone());
            continue;
        } else {
            let time_diff = (hour.0 - series[index - 1].0).num_hours();
            let time_diff = if time_diff > 1 && time_diff < MAX_HOUR_DIFF {
                time_diff
            } else if time_diff >= max_hour_diff {
                max_hour_diff
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

#[must_use]
fn locate_next_of_front_small_ones(series: &[(NaiveDateTime, usize)]) -> usize {
    let mut max: usize = 0;
    for s in series.iter() {
        if s.1 > max {
            max = s.1;
        }
    }

    let barrier = (max.to_f64().unwrap_or(1_000_000.0) / 10.0)
        .to_usize()
        .unwrap_or(5_usize);

    let mut location: usize = 0;
    let mut is_zero = false;
    for (index, s) in series.iter().enumerate() {
        if s.1 > barrier {
            location = index;
            break;
        }
        if s.1 > 0 {
            is_zero = true;
        }
    }

    if location > 2 || is_zero {
        location
    } else {
        0
    }
}
