use actix_web::{
    http,
    web::{Data, Json, Query},
    HttpResponse,
};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use structured::{Description, DescriptionElement};

use super::schema::{
    self, cluster, column_description, data_source, description_datetime, description_enum,
    description_float, description_int, description_ipaddr, description_text, top_n_datetime,
    top_n_enum, top_n_float, top_n_int, top_n_ipaddr, top_n_text,
};
use crate::database::{self, build_err_msg};

#[derive(Clone, Debug, Default, Serialize)]
struct DescriptionLoad {
    pub count: usize,
    pub unique_count: usize,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub min: Option<DescriptionElement>,
    pub max: Option<DescriptionElement>,
    pub top_n: Option<Vec<(DescriptionElement, usize)>>,
    pub mode: Option<DescriptionElement>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct DescriptionInsert {
    pub cluster_id: String,          // NOT cluster_id but id of cluster table
    pub first_event_id: Option<u64>, // String in other places
    pub last_event_id: Option<u64>,  // String in other places
    pub descriptions: Vec<Description>,
}

#[derive(Debug, Queryable)]
struct ColumnDescriptionsTable {
    // for loading from db
    pub id: i32,
    pub cluster_id: i32,
    pub first_event_id: String,
    pub last_event_id: String,
    pub column_index: i32,
    pub type_id: i32,
    pub count: i64,
    pub unique_count: i64,
}

#[derive(Debug, Insertable)]
#[table_name = "column_description"]
struct ColumnDescriptionsInsert {
    // for inserting into db
    pub cluster_id: i32,
    pub first_event_id: String,
    pub last_event_id: String,
    pub column_index: i32,
    pub type_id: i32,
    pub count: i64,
    pub unique_count: i64,
}

#[derive(Debug)]
struct ColumnDescriptionsWithSerial {
    // used during processing
    pub id: i32,
    pub cluster_id: i32,
    pub first_event_id: String,
    pub last_event_id: String,
    pub column_index: i32,
    pub type_id: i32,
    pub count: i64,
    pub unique_count: i64,
}

#[derive(Debug, Queryable)]
struct DescriptionsIntTable {
    // for loading from db
    pub id: i32,
    pub description_id: i32,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_int"]
struct DescriptionsIntInsert {
    // for inserting into db
    pub description_id: i32,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode: Option<i64>,
}

#[derive(Debug, Queryable)]
struct DescriptionsEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_enum"]
struct DescriptionsEnumInsert {
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Queryable)]
struct DescriptionsFloatTable {
    pub id: i32,
    pub description_id: i32,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode_smallest: Option<f64>,
    pub mode_largest: Option<f64>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_float"]
struct DescriptionsFloatInsert {
    pub description_id: i32,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode_smallest: Option<f64>,
    pub mode_largest: Option<f64>,
}

#[derive(Debug, Queryable)]
struct DescriptionsTextTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_text"]
struct DescriptionsTextInsert {
    pub description_id: i32,
    pub mode: Option<String>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Queryable)]
struct DescriptionsIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_ipaddr"]
struct DescriptionsIpaddrInsert {
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Queryable)]
struct DescriptionsDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[table_name = "description_datetime"]
struct DescriptionsDatetimeInsert {
    pub description_id: i32,
    pub mode: Option<NaiveDateTime>,
}

#[derive(Debug, Queryable)]
struct TopNIntTable {
    // for loading from db
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_int"]
struct TopNIntInsert {
    // for inserting into db
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_enum"]
struct TopNEnumInsert {
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNFloatTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value_smallest: Option<f64>,
    pub value_largest: Option<f64>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_float"]
struct TopNFloatInsert {
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value_smallest: Option<f64>,
    pub value_largest: Option<f64>,
    pub count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNTextTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_text"]
struct TopNTextInsert {
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Queryable)]
struct TopNIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_ipaddr"]
struct TopNIpaddrInsert {
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Queryable)]
struct TopNDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<NaiveDateTime>,
    pub count: Option<i64>,
}

#[derive(Debug, Insertable)]
#[table_name = "top_n_datetime"]
struct TopNDatetimeInsert {
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
struct RoundResponse {
    pub first_event_id: String,
    pub last_event_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DescriptionSelectQuery {
    pub cluster_id: String,
    pub data_source: String,
    pub first_event_id: String,
    pub last_event_id: String,
}

#[derive(Debug, Queryable)]
struct ColumnDescription {
    pub id: i32,
    pub column_index: i32,
    pub type_id: i32,
    pub count: i64,
    pub unique_count: i64,
}

macro_rules! push_top_n_others {
    ($cdsc:expr, $t:path, $func:tt, $id:expr, $d:expr, $dv:ident) => {{
        if let Some(top_n) = $cdsc.get_top_n() {
            for (r, (v, c)) in top_n.iter().enumerate() {
                let insert_v = if let $t(v) = v {
                    Some((*v).$func())
                } else {
                    None
                };
                $d.push($dv {
                    description_id: $id,
                    ranking: Some(safe_cast_usize_to_i64(r + 1)),
                    value: insert_v,
                    count: Some(safe_cast_usize_to_i64(*c)),
                });
            }
        }
    }};
}

macro_rules! insert_rows_to_db {
    ($r:expr, $conn:expr, $dd:ident, $td:ident, $dv:expr, $tv:expr) => {{
        $r.push(
            diesel::insert_into(schema::$dd::dsl::$dd)
                .values($dv)
                .execute($conn)
                .map_err(Into::into),
        );
        $r.push(
            diesel::insert_into(schema::$td::dsl::$td)
                .values($tv)
                .execute($conn)
                .map_err(Into::into),
        );
    }};
}

macro_rules! change_to_db_id {
    ($s:expr, $d:expr, $data:expr) => {{
        for d in $data.iter_mut() {
            d.description_id = *$d
                .get($s.get(&d.description_id).expect("safe"))
                .expect("safe");
        }
    }};
}

const MAX_VALUE_OF_I64_I64: i64 = 9_223_372_036_854_775_807_i64;
const MAX_VALUE_OF_I64_USIZE: usize = 9_223_372_036_854_775_807_usize;
const MAX_VALUE_OF_I32_USIZE: usize = 2_147_483_647_usize;

pub fn safe_cast_usize_to_i64(value: usize) -> i64 {
    if value > MAX_VALUE_OF_I64_USIZE {
        MAX_VALUE_OF_I64_I64
    } else {
        value.to_i64().expect("Safe cast: 0..=MaxOfi64 -> i64")
    }
}

#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
pub(crate) async fn add_descriptions(
    pool: Data<database::Pool>,
    new_descriptions: Json<Vec<DescriptionInsert>>,
) -> Result<HttpResponse, actix_web::Error> {
    let insert_descriptions: Vec<_> = new_descriptions.into_inner();

    let insert_result = pool.get().map_err(Into::into).and_then(|conn| {
        let mut result: Vec<Result<usize, database::Error>> = Vec::new();

        let mut desc_data_int = Vec::<DescriptionsIntInsert>::new();
        let mut top_n_data_int = Vec::<TopNIntInsert>::new();
        let mut desc_data_float = Vec::<DescriptionsFloatInsert>::new();
        let mut top_n_data_float = Vec::<TopNFloatInsert>::new();
        let mut desc_data_enum = Vec::<DescriptionsEnumInsert>::new();
        let mut top_n_data_enum = Vec::<TopNEnumInsert>::new();
        let mut desc_data_text = Vec::<DescriptionsTextInsert>::new();
        let mut top_n_data_text = Vec::<TopNTextInsert>::new();
        let mut desc_data_ipaddr = Vec::<DescriptionsIpaddrInsert>::new();
        let mut top_n_data_ipaddr = Vec::<TopNIpaddrInsert>::new();
        let mut desc_data_datetime = Vec::<DescriptionsDatetimeInsert>::new();
        let mut top_n_data_datetime = Vec::<TopNDatetimeInsert>::new();
        let mut column_description_data = Vec::<ColumnDescriptionsWithSerial>::new();

        let mut tmp_serial_id = 0_i32;

        for descriptions_of_cluster in insert_descriptions {
            let cluster_id = {
                use schema::cluster::dsl;
                match dsl::cluster
                    .filter(dsl::cluster_id.eq(&descriptions_of_cluster.cluster_id))
                    .select(dsl::id)
                    .first::<i32>(&conn)
                {
                    Ok(v) => v,
                    Err(_) => continue,
                }
            };

            for (i, column_description) in descriptions_of_cluster.descriptions.iter().enumerate() {
                let count = safe_cast_usize_to_i64(column_description.get_count());
                let unique_count = safe_cast_usize_to_i64(column_description.get_unique_count());

                let type_id: i32 = match column_description.get_mode() {
                    // TODO: Do I have to read this value from description_element_type table?
                    Some(DescriptionElement::Int(_)) => 1_i32,
                    Some(DescriptionElement::Enum(_)) => 2_i32,
                    Some(DescriptionElement::FloatRange(_, _)) => 3_i32,
                    Some(DescriptionElement::Text(_)) => 4_i32,
                    Some(DescriptionElement::IpAddr(_)) => 5_i32,
                    Some(DescriptionElement::DateTime(_)) => 6_i32,
                    _ => continue,
                };

                tmp_serial_id += 1;

                let first_event_id = if let Some(v) = descriptions_of_cluster.first_event_id {
                    v.to_string()
                } else {
                    continue;
                };
                let last_event_id = if let Some(v) = descriptions_of_cluster.last_event_id {
                    v.to_string()
                } else {
                    continue;
                };
                let column_index = if i <= MAX_VALUE_OF_I32_USIZE {
                    i.to_i32().expect("Safe cast: 0..=MaxOfi32 -> i32")
                } else {
                    continue;
                };
                column_description_data.push(ColumnDescriptionsWithSerial {
                    id: tmp_serial_id,
                    cluster_id,
                    first_event_id,
                    last_event_id,
                    column_index,
                    type_id,
                    count,
                    unique_count,
                });

                match column_description.get_mode() {
                    Some(DescriptionElement::Int(md)) => {
                        {
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
                            desc_data_int.push(DescriptionsIntInsert {
                                description_id: tmp_serial_id,
                                min: insert_min,
                                max: insert_max,
                                mean: column_description.get_mean(),
                                s_deviation: column_description.get_s_deviation(),
                                mode: Some(*md),
                            });
                        }
                        if let Some(top_n) = column_description.get_top_n() {
                            for (r, (v, c)) in top_n.iter().enumerate() {
                                let insert_v = match v {
                                    DescriptionElement::Int(v) => Some(*v),
                                    _ => None,
                                };
                                top_n_data_int.push(TopNIntInsert {
                                    description_id: tmp_serial_id,
                                    ranking: Some(safe_cast_usize_to_i64(r + 1)),
                                    value: insert_v,
                                    count: Some(safe_cast_usize_to_i64(*c)),
                                });
                            }
                        }
                    }
                    Some(DescriptionElement::Enum(md)) => {
                        desc_data_enum.push(DescriptionsEnumInsert {
                            description_id: tmp_serial_id,
                            mode: Some(md.clone()),
                        });
                        push_top_n_others!(
                            column_description,
                            DescriptionElement::Enum,
                            clone,
                            tmp_serial_id,
                            top_n_data_enum,
                            TopNEnumInsert
                        );
                    }
                    Some(DescriptionElement::FloatRange(x, y)) => {
                        {
                            let i_min = if let Some(DescriptionElement::Float(x)) =
                                column_description.get_min()
                            {
                                Some(*x)
                            } else {
                                None
                            };
                            let i_max = if let Some(DescriptionElement::Float(x)) =
                                column_description.get_max()
                            {
                                Some(*x)
                            } else {
                                None
                            };
                            desc_data_float.push(DescriptionsFloatInsert {
                                description_id: tmp_serial_id,
                                min: i_min,
                                max: i_max,
                                mean: column_description.get_mean(),
                                s_deviation: column_description.get_s_deviation(),
                                mode_smallest: Some(*x),
                                mode_largest: Some(*y),
                            });
                        }
                        {
                            if let Some(top_n) = column_description.get_top_n() {
                                for (r, (v, c)) in top_n.iter().enumerate() {
                                    let (x, y) = if let DescriptionElement::FloatRange(x, y) = v {
                                        (Some(*x), Some(*y))
                                    } else {
                                        (None, None)
                                    };
                                    top_n_data_float.push(TopNFloatInsert {
                                        description_id: tmp_serial_id,
                                        ranking: Some(safe_cast_usize_to_i64(r + 1)),
                                        value_smallest: x,
                                        value_largest: y,
                                        count: Some(safe_cast_usize_to_i64(*c)),
                                    });
                                }
                            }
                        }
                    }
                    Some(DescriptionElement::Text(md)) => {
                        desc_data_text.push(DescriptionsTextInsert {
                            description_id: tmp_serial_id,
                            mode: Some(md.clone()),
                        });
                        push_top_n_others!(
                            column_description,
                            DescriptionElement::Text,
                            clone,
                            tmp_serial_id,
                            top_n_data_text,
                            TopNTextInsert
                        );
                    }
                    Some(DescriptionElement::IpAddr(md)) => {
                        desc_data_ipaddr.push(DescriptionsIpaddrInsert {
                            description_id: tmp_serial_id,
                            mode: Some(md.to_string()),
                        });
                        push_top_n_others!(
                            column_description,
                            DescriptionElement::IpAddr,
                            to_string,
                            tmp_serial_id,
                            top_n_data_ipaddr,
                            TopNIpaddrInsert
                        );
                    }
                    Some(DescriptionElement::DateTime(md)) => {
                        desc_data_datetime.push(DescriptionsDatetimeInsert {
                            description_id: tmp_serial_id,
                            mode: Some(*md),
                        });
                        push_top_n_others!(
                            column_description,
                            DescriptionElement::DateTime,
                            clone,
                            tmp_serial_id,
                            top_n_data_datetime,
                            TopNDatetimeInsert
                        );
                    }
                    _ => continue,
                }
            }
        }

        let mut c_result = 0_usize;
        let column_description_data_to_db: Vec<ColumnDescriptionsInsert> = column_description_data
            .iter()
            .map(|d| ColumnDescriptionsInsert {
                cluster_id: d.cluster_id,
                first_event_id: d.first_event_id.clone(),
                last_event_id: d.last_event_id.clone(),
                column_index: d.column_index,
                type_id: d.type_id,
                count: d.count,
                unique_count: d.unique_count,
            })
            .collect();
        let serial_id: HashMap<i32, (i32, String, String, i32)> = column_description_data
            .iter()
            .map(|d| {
                (
                    d.id,
                    (
                        d.cluster_id,
                        d.first_event_id.clone(),
                        d.last_event_id.clone(),
                        d.column_index,
                    ),
                )
            })
            .collect();

        match diesel::insert_into(schema::column_description::dsl::column_description)
            .values(column_description_data_to_db)
            .get_results::<ColumnDescriptionsTable>(&conn)
        {
            Ok(inserted) => {
                let db_id: HashMap<(i32, String, String, i32), i32> = inserted
                    .iter()
                    .map(|d| {
                        (
                            (
                                d.cluster_id,
                                d.first_event_id.clone(),
                                d.last_event_id.clone(),
                                d.column_index,
                            ),
                            d.id,
                        )
                    })
                    .collect();

                change_to_db_id!(serial_id, db_id, desc_data_int);
                change_to_db_id!(serial_id, db_id, desc_data_enum);
                change_to_db_id!(serial_id, db_id, desc_data_float);
                change_to_db_id!(serial_id, db_id, desc_data_text);
                change_to_db_id!(serial_id, db_id, desc_data_ipaddr);
                change_to_db_id!(serial_id, db_id, desc_data_datetime);

                change_to_db_id!(serial_id, db_id, top_n_data_int);
                change_to_db_id!(serial_id, db_id, top_n_data_enum);
                change_to_db_id!(serial_id, db_id, top_n_data_float);
                change_to_db_id!(serial_id, db_id, top_n_data_text);
                change_to_db_id!(serial_id, db_id, top_n_data_ipaddr);
                change_to_db_id!(serial_id, db_id, top_n_data_datetime);

                insert_rows_to_db!(
                    result,
                    &conn,
                    description_int,
                    top_n_int,
                    desc_data_int,
                    top_n_data_int
                );
                insert_rows_to_db!(
                    result,
                    &conn,
                    description_enum,
                    top_n_enum,
                    desc_data_enum,
                    top_n_data_enum
                );
                insert_rows_to_db!(
                    result,
                    &conn,
                    description_float,
                    top_n_float,
                    desc_data_float,
                    top_n_data_float
                );
                insert_rows_to_db!(
                    result,
                    &conn,
                    description_text,
                    top_n_text,
                    desc_data_text,
                    top_n_data_text
                );
                insert_rows_to_db!(
                    result,
                    &conn,
                    description_ipaddr,
                    top_n_ipaddr,
                    desc_data_ipaddr,
                    top_n_data_ipaddr
                );
                insert_rows_to_db!(
                    result,
                    &conn,
                    description_datetime,
                    top_n_datetime,
                    desc_data_datetime,
                    top_n_data_datetime
                );
            }
            Err(_) => c_result = 1_usize,
        }

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

    match insert_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

pub(crate) async fn get_rounds_by_cluster(
    pool: Data<database::Pool>,
    query: Query<RoundSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as cd_d;
    use data_source::dsl as d_d;
    let query_result: Result<Vec<RoundResponse>, database::Error> =
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

    match query_result {
        Ok(round_response) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(round_response)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}

struct GetValueByType {}

impl GetValueByType {
    pub fn top_n_enum(value: &Option<String>) -> DescriptionElement {
        match value {
            Some(value) => DescriptionElement::Enum(value.clone()),
            None => DescriptionElement::Enum(String::from("N/A")), // No chance
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

    pub fn mode_enum(value: Option<String>) -> Option<DescriptionElement> {
        match value {
            Some(m) => Some(DescriptionElement::Enum(m)),
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
                            Some(count) => count.to_usize().unwrap_or(0_usize),
                            None => 0_usize, // No chance
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
                count: $column.count.to_usize().unwrap_or(0_usize),
                unique_count: $column.unique_count.to_usize().unwrap_or(0_usize),
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

#[allow(clippy::too_many_lines)]
pub(crate) async fn get_description(
    pool: Data<database::Pool>,
    query: Query<DescriptionSelectQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    use cluster::dsl as c_d;
    use column_description::dsl as cd_d;
    use data_source::dsl as d_d;
    let query_result: Result<Vec<DescriptionLoad>, database::Error> =
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
                                                    Some(count) => {
                                                        count.to_usize().unwrap_or(0_usize)
                                                    }
                                                    None => 0_usize, // No chance
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
                                        count: column.count.to_usize().unwrap_or(0_usize),
                                        unique_count: column
                                            .unique_count
                                            .to_usize()
                                            .unwrap_or(0_usize),
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
                                    String,
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
                                                    Some(count) => {
                                                        count.to_usize().unwrap_or(0_usize)
                                                    }
                                                    None => 0_usize, // No chance
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
                                        count: column.count.to_usize().unwrap_or(0_usize),
                                        unique_count: column
                                            .unique_count
                                            .to_usize()
                                            .unwrap_or(0_usize),
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

    match query_result {
        Ok(response) => Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    }
}
