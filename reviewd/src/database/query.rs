use actix_web::{http, web::Query, HttpResponse};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::{BigInt, Jsonb};
use serde::Deserialize;
use serde_json::Value;

use crate::database::{build_err_msg, Error};

#[derive(Debug, Deserialize, QueryableByName)]
pub(crate) struct GetQueryData {
    #[sql_type = "Jsonb"]
    pub(crate) data: Value,
    #[sql_type = "BigInt"]
    pub(crate) count: i64,
}

#[derive(Debug)]
pub(crate) struct GetQuery<'a> {
    pub(crate) select: Vec<&'a str>,
    pub(crate) schema: &'a str,
    pub(crate) where_clause: Option<String>,
    pub(crate) page: Option<i64>,
    pub(crate) per_page: i64,
    pub(crate) orderby: Option<&'a str>,
    pub(crate) order: Option<&'a str>,
}

impl<'a> GetQuery<'a> {
    pub(crate) fn new(
        select: Vec<&'a str>,
        schema: &'a str,
        where_clause: Option<String>,
        page: Option<i64>,
        per_page: i64,
        orderby: Option<&'a str>,
        order: Option<&'a str>,
    ) -> Self {
        Self {
            select,
            schema,
            where_clause,
            page,
            per_page,
            orderby,
            order,
        }
    }

    pub(crate) fn build_response(
        query: &Query<Value>,
        per_page: i64,
        query_result: Result<Vec<GetQueryData>, Error>,
    ) -> Result<HttpResponse, actix_web::Error> {
        match query_result {
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
        }
    }

    pub(crate) fn get_order(query: &Query<Value>) -> Option<&'a str> {
        query
            .get("order")
            .and_then(Value::as_str)
            .and_then(|order| match order.to_lowercase().as_str() {
                "desc" => Some("desc"),
                _ => None,
            })
    }

    pub(crate) fn get_page(query: &Query<Value>) -> Option<i64> {
        query
            .get("page")
            .and_then(Value::as_str)
            .and_then(|p| p.parse::<i64>().ok())
            .filter(|p| *p > 0)
    }

    pub(crate) fn get_per_page(query: &Query<Value>, max_per_page: i64) -> Option<i64> {
        query
            .get("per_page")
            .and_then(Value::as_str)
            .and_then(|p| p.parse::<i64>().ok())
            .filter(|p| *p > 0)
            .map(|p| if p > max_per_page { max_per_page } else { p })
    }
}

impl<'a> QueryFragment<Pg> for GetQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_sql("SELECT to_jsonb(a) as data, b.count FROM ( SELECT ");
        for (i, column) in self.select.iter().enumerate() {
            out.push_sql(column);
            if i < self.select.len() - 1 {
                out.push_sql(", ");
            } else {
                out.push_sql(" FROM ");
            }
        }
        out.push_sql(self.schema);
        if let Some(where_clause) = &self.where_clause {
            out.push_sql(" WHERE ");
            out.push_sql(&where_clause);
        }
        if let Some(orderby) = &self.orderby {
            out.push_sql(" ORDER BY ");
            out.push_sql(orderby);
        }
        if let Some(order) = &self.order {
            out.push_sql(" ");
            out.push_sql(order);
        }
        out.push_sql(" LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        if let Some(page) = &self.page {
            let offset = (page - 1) * self.per_page;
            out.push_sql(" OFFSET ");
            out.push_bind_param::<BigInt, _>(&offset)?;
        }
        out.push_sql(") as a, (SELECT *, COUNT(*) OVER() FROM ");
        out.push_sql(self.schema);
        out.push_sql(" LIMIT 1) b");
        Ok(())
    }
}

impl<'a> QueryId for GetQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a> LoadQuery<PgConnection, GetQueryData> for GetQuery<'a> {
    fn internal_load(self, conn: &PgConnection) -> QueryResult<Vec<GetQueryData>> {
        conn.query_by_name(&self)
    }
}

impl<'a, Conn> RunQueryDsl<Conn> for GetQuery<'a> {}
