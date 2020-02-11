use actix_web::{
    http,
    web::{Data, Query},
    HttpResponse,
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::schema::{cluster, data_source};
use crate::database::{self, build_err_msg};

#[derive(Debug, Deserialize)]
pub(crate) struct SizeSelectQuery {
    data_source: String,
}

#[derive(Debug, Queryable, Serialize)]
struct SizeOfClustersResponse {
    pub size: BigDecimal,
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
