use actix_web::{
    http,
    web::{Data, Json, Query},
    HttpResponse,
};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use futures::{future, prelude::*};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use structured::{Description, DescriptionElement};

use super::schema;
use super::schema::{
    cluster, column_description, data_source, description_datetime, description_element_type,
    description_enum, description_float, description_int, description_ipaddr, description_text,
    top_n_datetime, top_n_enum, top_n_float, top_n_int, top_n_ipaddr, top_n_text,
};
use crate::database::*;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct DescriptionLoad {
    pub count: usize,
    pub unique_count: usize,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub min: Option<DescriptionElement>,
    pub max: Option<DescriptionElement>,
    pub top_n: Option<Vec<(DescriptionElement, usize)>>,
    pub mode: Option<DescriptionElement>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DescriptionUpdate {
    pub cluster_id: String,          // NOT cluster_id but id of cluster table
    pub first_event_id: Option<u64>, // String in other places
    pub last_event_id: Option<u64>,  // String in other places
    pub descriptions: Vec<Description>,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_element_type"]
pub(crate) struct DescriptionElementTypeTable {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "column_description"]
pub(crate) struct ColumnDescriptionsTable {
    pub id: i32,
    pub cluster_id: i32,
    pub first_event_id: String,
    pub last_event_id: String,
    pub column_index: i32,
    pub type_id: i32,
    pub count: i64,
    pub unique_count: i64,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_int"]
pub(crate) struct DescriptionsIntTable {
    pub id: i32,
    pub description_id: i32,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_enum"]
pub(crate) struct DescriptionsEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_float"]
pub(crate) struct DescriptionsFloatTable {
    pub id: i32,
    pub description_id: i32,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode_smallest: Option<f64>,
    pub mode_largest: Option<f64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_text"]
pub(crate) struct DescriptionsTextTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_ipaddr"]
pub(crate) struct DescriptionsIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_datetime"]
pub(crate) struct DescriptionsDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_int"]
pub(crate) struct TopNIntTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_enum"]
pub(crate) struct TopNEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_float"]
pub(crate) struct TopNFloatTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value_smallest: Option<f64>,
    pub value_largest: Option<f64>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_text"]
pub(crate) struct TopNTextTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_ipaddr"]
pub(crate) struct TopNIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_datetime"]
pub(crate) struct TopNDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<NaiveDateTime>,
    pub count: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoundSelectQuery {
    pub(crate) cluster_id: String,
    pub(crate) data_source: String,
}

#[derive(Debug, Queryable, Serialize)]
pub(crate) struct RoundResponse {
    pub(crate) first_event_id: String,
    pub(crate) last_event_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DescriptionSelectQuery {
    pub(crate) cluster_id: String,
    pub(crate) data_source: String,
    pub(crate) first_event_id: String,
    pub(crate) last_event_id: String,
}

#[derive(Debug, Queryable, Serialize)]
pub(crate) struct ColumnDescription {
    pub(crate) id: i32,
    pub(crate) column_index: i32,
    pub(crate) type_id: i32,
    pub(crate) count: i64,
    pub(crate) unique_count: i64,
}

macro_rules! insert_top_n_number {
    ($p:ident, $cdsc:expr, $id:expr, $conn:expr, $re:expr) => {{
        use schema::$p::*;
        if let Some(top_n) = $cdsc.get_top_n() {
            for (r, (v, c)) in top_n.iter().enumerate() {
                let insert_v = match v {
                    DescriptionElement::Int(v) => Some(*v),
                    DescriptionElement::UInt(v) => {
                        Some(v.to_i64().expect("Safe cast: 0..=MaxOfu32 -> i64"))
                    }
                    _ => None,
                };
                $re.push(
                    diesel::insert_into(dsl::$p)
                        .values((
                            dsl::description_id.eq($id),
                            dsl::ranking.eq(Some(safe_cast_usize_to_i64(r + 1))),
                            dsl::value.eq(insert_v),
                            dsl::count.eq(Some(safe_cast_usize_to_i64(*c))),
                        ))
                        .execute($conn)
                        .map_err(Into::into),
                );
            }
        }
    }};
}

macro_rules! insert_top_n_others {
    ($p:ident, $cdsc:expr, $t:path, $func:tt, $id:expr, $conn:expr, $re:expr) => {{
        use schema::$p::*;
        if let Some(top_n) = $cdsc.get_top_n() {
            for (r, (v, c)) in top_n.iter().enumerate() {
                let insert_v = if let $t(v) = v {
                    Some((*v).$func())
                } else {
                    None
                };
                $re.push(
                    diesel::insert_into(dsl::$p)
                        .values((
                            dsl::description_id.eq($id),
                            dsl::ranking.eq(Some(safe_cast_usize_to_i64(r + 1))),
                            dsl::value.eq(insert_v),
                            dsl::count.eq(Some(safe_cast_usize_to_i64(*c))),
                        ))
                        .execute($conn)
                        .map_err(Into::into),
                );
            }
        }
    }};
}

macro_rules! insert_short_description {
    ($p:ident, $id:expr, $md:expr, $conn:expr, $re:expr) => {{
        use schema::$p::*;
        $re.push(
            diesel::insert_into(dsl::$p)
                .values((dsl::description_id.eq($id), dsl::mode.eq($md)))
                .execute($conn)
                .map_err(Into::into),
        );
    }};
}

const MAX_VALUE_OF_I64_I64: i64 = 9_223_372_036_854_775_807_i64;
const MAX_VALUE_OF_I64_USIZE: usize = 9_223_372_036_854_775_807_usize;
const MAX_VALUE_OF_I32_USIZE: usize = 2_147_483_647_usize;
const MAX_VALUE_OF_U32_I64: i64 = 4_294_967_295_i64;
const MAX_VALUE_OF_U32_U32: u32 = 4_294_967_295_u32;

pub fn safe_cast_usize_to_i64(value: usize) -> i64 {
    if value > MAX_VALUE_OF_I64_USIZE {
        MAX_VALUE_OF_I64_I64
    } else {
        value.to_i64().expect("Safe cast: 0..=MaxOfi64 -> i64")
    }
}

#[allow(clippy::cognitive_complexity)]
pub(crate) fn add_descriptions(
    pool: Data<Pool>,
    new_descriptions: Json<Vec<DescriptionUpdate>>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let insert_descriptions: Vec<_> = new_descriptions.into_inner();

    let insert_result: Result<usize, Error> = pool.get().map_err(Into::into).and_then(|conn| {
        let mut result: Vec<Result<usize, Error>> = Vec::new();
        for descriptions_of_cluster in insert_descriptions {
            let cid;
            {
                use schema::cluster::*;
                match dsl::cluster
                    .filter(dsl::cluster_id.eq(&descriptions_of_cluster.cluster_id))
                    .select(dsl::id)
                    .first::<i32>(&conn)
                {
                    Ok(v) => cid = v,
                    Err(_) => continue,
                }
            }

            for (i, column_description) in descriptions_of_cluster.descriptions.iter().enumerate() {
                let cnt = safe_cast_usize_to_i64(column_description.get_count());
                let unique_cnt = safe_cast_usize_to_i64(column_description.get_unique_count());

                let tid: i32 = match column_description.get_mode() {
                    // TODO: Do I have to read this value from description_element_type table?
                    Some(DescriptionElement::Int(_)) => 1_i32,
                    Some(DescriptionElement::UInt(_)) => 2_i32,
                    Some(DescriptionElement::Float(_)) => continue,
                    Some(DescriptionElement::FloatRange(_, _)) => 3_i32,
                    Some(DescriptionElement::Text(_)) => 4_i32,
                    Some(DescriptionElement::IpAddr(_)) => 5_i32,
                    Some(DescriptionElement::DateTime(_)) => 6_i32,
                    None => continue,
                };

                let inserted: ColumnDescriptionsTable;
                {
                    use schema::column_description::*;

                    let first_e_i = if let Some(v) = descriptions_of_cluster.first_event_id {
                        v.to_string()
                    } else {
                        continue;
                    };
                    let last_e_i = if let Some(v) = descriptions_of_cluster.last_event_id {
                        v.to_string()
                    } else {
                        continue;
                    };
                    let c_index = if i <= MAX_VALUE_OF_I32_USIZE {
                        i.to_i32().expect("Safe cast: 0..=MaxOfi32 -> i32")
                    } else {
                        continue;
                    };
                    inserted = match diesel::insert_into(dsl::column_description)
                        .values((
                            dsl::cluster_id.eq(cid),
                            dsl::first_event_id.eq(first_e_i),
                            dsl::last_event_id.eq(last_e_i),
                            dsl::column_index.eq(c_index),
                            dsl::type_id.eq(tid),
                            dsl::count.eq(cnt),
                            dsl::unique_count.eq(unique_cnt),
                        ))
                        .get_result(&conn)
                    {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                }

                match column_description.get_mode() {
                    Some(DescriptionElement::Int(md)) => {
                        {
                            use schema::description_int::*;
                            let insert_min = if let Some(DescriptionElement::Int(v)) =
                                column_description.get_min()
                            {
                                Some(*v)
                            } else {
                                None
                            };
                            let insert_max = if let Some(DescriptionElement::Int(v)) =
                                column_description.get_max()
                            {
                                Some(*v)
                            } else {
                                None
                            };
                            result.push(
                                diesel::insert_into(dsl::description_int)
                                    .values((
                                        dsl::description_id.eq(inserted.id),
                                        dsl::min.eq(insert_min),
                                        dsl::max.eq(insert_max),
                                        dsl::mean.eq(column_description.get_mean()),
                                        dsl::s_deviation.eq(column_description.get_s_deviation()),
                                        dsl::mode.eq(Some(*md)),
                                    ))
                                    .execute(&conn)
                                    .map_err(Into::into),
                            );
                        }
                        insert_top_n_number!(
                            top_n_int,
                            column_description,
                            inserted.id,
                            &conn,
                            result
                        );
                    }
                    Some(DescriptionElement::UInt(md)) => {
                        insert_short_description!(
                            description_enum,
                            inserted.id,
                            Some(md.to_i64().expect("Safe cast: 0..=MaxOfu32 -> i64")),
                            &conn,
                            result
                        );
                        insert_top_n_number!(
                            top_n_enum,
                            column_description,
                            inserted.id,
                            &conn,
                            result
                        );
                    }
                    Some(DescriptionElement::Float(_)) => (),
                    Some(DescriptionElement::FloatRange(x, y)) => {
                        {
                            use schema::description_float::*;
                            let i_min = if let Some(DescriptionElement::Float(x)) =
                                column_description.get_min()
                            {
                                Some(x)
                            } else {
                                None
                            };
                            let i_max = if let Some(DescriptionElement::Float(x)) =
                                column_description.get_max()
                            {
                                Some(x)
                            } else {
                                None
                            };
                            result.push(
                                diesel::insert_into(dsl::description_float)
                                    .values((
                                        dsl::description_id.eq(inserted.id),
                                        dsl::min.eq(i_min),
                                        dsl::max.eq(i_max),
                                        dsl::mean.eq(column_description.get_mean()),
                                        dsl::s_deviation.eq(column_description.get_s_deviation()),
                                        dsl::mode_smallest.eq(Some(*x)),
                                        dsl::mode_largest.eq(Some(*y)),
                                    ))
                                    .execute(&conn)
                                    .map_err(Into::into),
                            );
                        }
                        {
                            use schema::top_n_float::*;
                            if let Some(top_n) = column_description.get_top_n() {
                                for (r, (v, c)) in top_n.iter().enumerate() {
                                    let (x, y) = if let DescriptionElement::FloatRange(x, y) = v {
                                        (Some(*x), Some(*y))
                                    } else {
                                        (None, None)
                                    };
                                    result.push(
                                        diesel::insert_into(dsl::top_n_float)
                                            .values((
                                                dsl::description_id.eq(inserted.id),
                                                dsl::ranking
                                                    .eq(Some(safe_cast_usize_to_i64(r + 1))),
                                                dsl::value_smallest.eq(x),
                                                dsl::value_largest.eq(y),
                                                dsl::count.eq(Some(safe_cast_usize_to_i64(*c))),
                                            ))
                                            .execute(&conn)
                                            .map_err(Into::into),
                                    );
                                }
                            }
                        }
                    }
                    Some(DescriptionElement::Text(md)) => {
                        insert_short_description!(
                            description_text,
                            inserted.id,
                            Some(md.clone()),
                            &conn,
                            result
                        );
                        insert_top_n_others!(
                            top_n_text,
                            column_description,
                            DescriptionElement::Text,
                            clone,
                            inserted.id,
                            &conn,
                            result
                        );
                    }
                    Some(DescriptionElement::IpAddr(md)) => {
                        insert_short_description!(
                            description_ipaddr,
                            inserted.id,
                            Some(md.to_string()),
                            &conn,
                            result
                        );
                        insert_top_n_others!(
                            top_n_ipaddr,
                            column_description,
                            DescriptionElement::IpAddr,
                            to_string,
                            inserted.id,
                            &conn,
                            result
                        );
                    }
                    Some(DescriptionElement::DateTime(md)) => {
                        insert_short_description!(
                            description_datetime,
                            inserted.id,
                            Some(*md),
                            &conn,
                            result
                        );
                        insert_top_n_others!(
                            top_n_datetime,
                            column_description,
                            DescriptionElement::DateTime,
                            clone,
                            inserted.id,
                            &conn,
                            result
                        );
                    }
                    None => continue,
                }
            }
        }

        let mut c_result = 0_usize;
        for r in result {
            match r {
                Ok(_) => {
                    c_result += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(c_result)
    });

    let result = match insert_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

pub(crate) fn get_rounds_by_cluster(
    pool: Data<Pool>,
    query: Query<RoundSelectQuery>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as cd_d;
    use data_source::dsl as d_d;
    let query_result: Result<Vec<RoundResponse>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            cd_d::column_description
                .inner_join(c_d::cluster.on(cd_d::cluster_id.eq(c_d::id)))
                .inner_join(d_d::data_source.on(c_d::data_source_id.eq(d_d::id)))
                .select((cd_d::first_event_id, cd_d::last_event_id))
                .filter(
                    c_d::cluster_id
                        .eq(&query.cluster_id)
                        .and(d_d::topic_name.eq(&query.data_source)),
                )
                .distinct_on((cd_d::first_event_id, cd_d::last_event_id))
                .load::<RoundResponse>(&conn)
                .map_err(Into::into)
        });

    let result = match query_result {
        Ok(round_response) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(round_response)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}

struct GetValueByType {}

impl GetValueByType {
    pub fn top_n_enum(value: &Option<i64>) -> DescriptionElement {
        match value {
            Some(value) => {
                if *value <= MAX_VALUE_OF_U32_I64 {
                    DescriptionElement::UInt(
                        value.to_u32().expect("Safe cast: 0..=MaxOfu32 -> u32"),
                    )
                } else {
                    DescriptionElement::UInt(MAX_VALUE_OF_U32_U32)
                }
            }
            None => DescriptionElement::UInt(0_u32), // No chance
        }
    }

    pub fn top_n_text(value: &Option<String>) -> DescriptionElement {
        match value {
            Some(value) => DescriptionElement::Text(value.clone()),
            None => DescriptionElement::Text(String::from("N/A")), // No chance
        }
    }

    pub fn top_n_ipaddr(value: &Option<String>) -> DescriptionElement {
        match value {
            Some(value) => DescriptionElement::IpAddr(IpAddr::V4(
                Ipv4Addr::from_str(value).unwrap_or_else(|_| Ipv4Addr::new(0, 0, 0, 0)),
            )),
            None => DescriptionElement::IpAddr(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))), // No chance
        }
    }

    pub fn top_n_datetime(value: &Option<NaiveDateTime>) -> DescriptionElement {
        match value {
            Some(value) => DescriptionElement::DateTime(*value),
            None => DescriptionElement::DateTime(NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1)), // No chance
        }
    }

    pub fn mode_enum(value: Option<i64>) -> Option<DescriptionElement> {
        match value {
            Some(m) => {
                if m <= MAX_VALUE_OF_U32_I64 {
                    Some(DescriptionElement::UInt(
                        m.to_u32().expect("Safe cast: 0..=MaxOfu32 -> u32"),
                    ))
                } else {
                    Some(DescriptionElement::UInt(MAX_VALUE_OF_U32_U32)) // No chance
                }
            }
            None => None,
        }
    }

    pub fn mode_text(value: Option<String>) -> Option<DescriptionElement> {
        match value {
            Some(m) => Some(DescriptionElement::Text(m)),
            None => None,
        }
    }

    pub fn mode_ipaddr(value: Option<String>) -> Option<DescriptionElement> {
        match value {
            Some(m) => Some(DescriptionElement::IpAddr(IpAddr::V4(
                Ipv4Addr::from_str(&m).unwrap_or_else(|_| Ipv4Addr::new(0, 0, 0, 0)),
            ))),
            None => None,
        }
    }

    pub fn mode_datetime(value: Option<NaiveDateTime>) -> Option<DescriptionElement> {
        match value {
            Some(m) => Some(DescriptionElement::DateTime(m)),
            None => None,
        }
    }
}

macro_rules! load_descriptions_others {
    ($s_desc:ident, $s_top_n:ident, $t_desc:ty, $t_top_n:ty,
        $conn:expr, $column:expr, $description:expr, $value_top_n:tt, $value_mode:tt) => {{
        use $s_desc::dsl as d_d;
        if let Ok(mode) = d_d::$s_desc
            .select(d_d::mode)
            .filter(d_d::description_id.eq(&$column.id))
            .first::<Option<$t_desc>>($conn)
        {
            use $s_top_n::dsl as t_d;
            let top_n = if let Ok(top_n) = t_d::$s_top_n
                .filter(t_d::description_id.eq(&$column.id))
                .order_by(t_d::ranking.asc())
                .load::<$t_top_n>($conn)
            {
                let top_n: Vec<(DescriptionElement, usize)> = top_n
                    .iter()
                    .map(|t| {
                        let value = GetValueByType::$value_top_n(&t.value);
                        let count = match t.count {
                            Some(count) => count as usize, // safe
                            None => 0_usize,               // No chance
                        };
                        (value, count)
                    })
                    .collect();
                Some(top_n)
            } else {
                None
            };
            let mode = GetValueByType::$value_mode(mode);
            $description = DescriptionLoad {
                count: $column.count as usize,               // safe
                unique_count: $column.unique_count as usize, // safe
                mean: None,
                s_deviation: None,
                min: None,
                max: None,
                top_n: top_n,
                mode: mode,
            }
        } else {
            $description = DescriptionLoad::default()
        }
    }};
}

pub(crate) fn get_description(
    pool: Data<Pool>,
    query: Query<DescriptionSelectQuery>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as cd_d;
    use data_source::dsl as d_d;
    let query_result: Result<Vec<DescriptionLoad>, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            cd_d::column_description
                .inner_join(c_d::cluster.on(cd_d::cluster_id.eq(c_d::id)))
                .inner_join(d_d::data_source.on(c_d::data_source_id.eq(d_d::id)))
                .select((
                    cd_d::id,
                    cd_d::column_index,
                    cd_d::type_id,
                    cd_d::count,
                    cd_d::unique_count,
                ))
                .filter(
                    c_d::cluster_id
                        .eq(&query.cluster_id)
                        .and(d_d::topic_name.eq(&query.data_source))
                        .and(cd_d::first_event_id.eq(&query.first_event_id))
                        .and(cd_d::last_event_id.eq(&query.last_event_id)),
                )
                .order_by(cd_d::column_index.asc())
                .load::<ColumnDescription>(&conn)
                .and_then(|columns| {
                    Ok(columns
                        .iter()
                        .map(|column| match column.type_id {
                            1 => {
                                use description_int::dsl as di_d;
                                if let Ok((min, max, mean, s_deviation, mode)) =
                                    di_d::description_int
                                        .select((
                                            di_d::min,
                                            di_d::max,
                                            di_d::mean,
                                            di_d::s_deviation,
                                            di_d::mode,
                                        ))
                                        .filter(di_d::description_id.eq(&column.id))
                                        .first::<(
                                            Option<i64>,
                                            Option<i64>,
                                            Option<f64>,
                                            Option<f64>,
                                            Option<i64>,
                                        )>(&conn)
                                {
                                    use top_n_int::dsl as ti_d;
                                    let top_n = if let Ok(top_n) = ti_d::top_n_int
                                        .filter(ti_d::description_id.eq(&column.id))
                                        .order_by(ti_d::ranking.asc())
                                        .load::<TopNIntTable>(&conn)
                                    {
                                        let top_n: Vec<(DescriptionElement, usize)> = top_n
                                            .iter()
                                            .map(|t| {
                                                let value = match t.value {
                                                    Some(value) => DescriptionElement::Int(value),
                                                    None => DescriptionElement::Int(0_i64), // No chance
                                                };
                                                let count = match t.count {
                                                    Some(count) => count as usize, // safe
                                                    None => 0_usize,               // No chance
                                                };
                                                (value, count)
                                            })
                                            .collect();
                                        Some(top_n)
                                    } else {
                                        None
                                    };
                                    let min = match min {
                                        Some(m) => Some(DescriptionElement::Int(m)),
                                        None => None,
                                    };
                                    let max = match max {
                                        Some(m) => Some(DescriptionElement::Int(m)),
                                        None => None,
                                    };
                                    let mode = match mode {
                                        Some(m) => Some(DescriptionElement::Int(m)),
                                        None => None,
                                    };
                                    DescriptionLoad {
                                        count: column.count as usize,               // safe
                                        unique_count: column.unique_count as usize, // safe
                                        mean,
                                        s_deviation,
                                        min,
                                        max,
                                        top_n,
                                        mode,
                                    }
                                } else {
                                    DescriptionLoad::default()
                                }
                            }
                            2 => {
                                let description;
                                load_descriptions_others!(
                                    description_enum,
                                    top_n_enum,
                                    i64,
                                    TopNEnumTable,
                                    &conn,
                                    column,
                                    description,
                                    top_n_enum,
                                    mode_enum
                                );
                                description
                            }
                            3 => {
                                use description_float::dsl as df_d;
                                if let Ok((
                                    min,
                                    max,
                                    mean,
                                    s_deviation,
                                    mode_smallest,
                                    mode_largest,
                                )) = df_d::description_float
                                    .select((
                                        df_d::min,
                                        df_d::max,
                                        df_d::mean,
                                        df_d::s_deviation,
                                        df_d::mode_smallest,
                                        df_d::mode_largest,
                                    ))
                                    .filter(df_d::description_id.eq(&column.id))
                                    .first::<(
                                        Option<f64>,
                                        Option<f64>,
                                        Option<f64>,
                                        Option<f64>,
                                        Option<f64>,
                                        Option<f64>,
                                    )>(&conn)
                                {
                                    use top_n_float::dsl as tf_d;
                                    let top_n = if let Ok(top_n) = tf_d::top_n_float
                                        .filter(tf_d::description_id.eq(&column.id))
                                        .order_by(tf_d::ranking.asc())
                                        .load::<TopNFloatTable>(&conn)
                                    {
                                        let top_n: Vec<(DescriptionElement, usize)> = top_n
                                            .iter()
                                            .map(|t| {
                                                let smallest = match t.value_smallest {
                                                    Some(value) => value,
                                                    None => 0_f64, // No chance
                                                };
                                                let largest = match t.value_largest {
                                                    Some(value) => value,
                                                    None => 0_f64, // No chance
                                                };
                                                let count = match t.count {
                                                    Some(count) => count as usize, // safe
                                                    None => 0_usize,               // No chance
                                                };
                                                (
                                                    DescriptionElement::FloatRange(
                                                        smallest, largest,
                                                    ),
                                                    count,
                                                )
                                            })
                                            .collect();
                                        Some(top_n)
                                    } else {
                                        None
                                    };
                                    let min = match min {
                                        Some(m) => Some(DescriptionElement::Float(m)),
                                        None => None,
                                    };
                                    let max = match max {
                                        Some(m) => Some(DescriptionElement::Float(m)),
                                        None => None,
                                    };
                                    let mode = if let (Some(smallest), Some(largest)) =
                                        (mode_smallest, mode_largest)
                                    {
                                        Some(DescriptionElement::FloatRange(smallest, largest))
                                    } else {
                                        None
                                    };
                                    DescriptionLoad {
                                        count: column.count as usize,               // safe
                                        unique_count: column.unique_count as usize, // safe
                                        mean,
                                        s_deviation,
                                        min,
                                        max,
                                        top_n,
                                        mode,
                                    }
                                } else {
                                    DescriptionLoad::default()
                                }
                            }
                            4 => {
                                let description;
                                load_descriptions_others!(
                                    description_text,
                                    top_n_text,
                                    String,
                                    TopNTextTable,
                                    &conn,
                                    column,
                                    description,
                                    top_n_text,
                                    mode_text
                                );
                                description
                            }
                            5 => {
                                let description;
                                load_descriptions_others!(
                                    description_ipaddr,
                                    top_n_ipaddr,
                                    String,
                                    TopNIpaddrTable,
                                    &conn,
                                    column,
                                    description,
                                    top_n_ipaddr,
                                    mode_ipaddr
                                );
                                description
                            }
                            6 => {
                                let description;
                                load_descriptions_others!(
                                    description_datetime,
                                    top_n_datetime,
                                    NaiveDateTime,
                                    TopNDatetimeTable,
                                    &conn,
                                    column,
                                    description,
                                    top_n_datetime,
                                    mode_datetime
                                );
                                description
                            }
                            _ => DescriptionLoad::default(),
                        })
                        .collect::<Vec<DescriptionLoad>>())
                })
                .map_err(Into::into)
        });

    let result = match query_result {
        Ok(response) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(result)
}
