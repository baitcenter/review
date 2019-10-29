use actix_web::{
    http,
    web::{Data, Json},
    HttpResponse,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use futures::{future, prelude::*};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use structured::{Description, DescriptionElement};

use super::schema;
use super::schema::{
    column_description, description_datetime, description_element_type, description_enum,
    description_float, description_int, description_ipaddr, description_text, top_n_datetime,
    top_n_enum, top_n_float, top_n_int, top_n_ipaddr, top_n_text,
};
use crate::database::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DescriptionUpdate {
    pub cluster_id: String,
    pub first_event_id: Option<u64>,
    pub last_event_id: Option<u64>,
    pub descriptions: Vec<Description>,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_element_type"]
pub(crate) struct DescriptionElementTypeTable {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "column_description"]
#[belongs_to(ClustersTable, foreign_key = "cluster_id")]
#[belongs_to(DescriptionElementTypeTable, foreign_key = "type_id")]
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

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_int"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct DescriptionsIntTable {
    pub id: i32,
    pub description_id: i32,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub mean: Option<f64>,
    pub s_deviation: Option<f64>,
    pub mode: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_enum"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct DescriptionsEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_float"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
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

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_text"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct DescriptionsTextTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_ipaddr"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct DescriptionsIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<String>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "description_datetime"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct DescriptionsDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub mode: Option<NaiveDateTime>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_int"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNIntTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_enum"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNEnumTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_float"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNFloatTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value_smallest: Option<f64>,
    pub value_largest: Option<f64>,
    pub count: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_text"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNTextTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

// TODO: Instead of String, how about using cidr? But, need to figure out how.
#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_ipaddr"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNIpaddrTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Associations, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "top_n_datetime"]
#[belongs_to(ColumnDescriptionsTable, foreign_key = "description_id")]
pub(crate) struct TopNDatetimeTable {
    pub id: i32,
    pub description_id: i32,
    pub ranking: Option<i64>,
    pub value: Option<NaiveDateTime>,
    pub count: Option<i64>,
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
                    Err(e) => return Err(e).map_err(Into::into),
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
                    let c_index = if i
                        <= i32::max_value()
                            .to_usize()
                            .expect("Safe cast: MaxOfi32 -> usize")
                    {
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
                        Err(e) => return Err(e).map_err(Into::into),
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

    let final_result = match insert_result {
        Ok(_) => Ok(HttpResponse::Ok().into()),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e))),
    };

    future::result(final_result)
}
