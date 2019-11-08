use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::{BigInt, Jsonb};
use serde::Deserialize;

#[derive(Debug, Deserialize, QueryableByName)]
pub(crate) struct GetQueryData {
    #[sql_type = "Jsonb"]
    pub(crate) data: serde_json::Value,
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
