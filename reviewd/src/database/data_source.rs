use super::schema::data_source;
use crate::database::{Error, Pool};
use actix_web::web;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct DataSourceQuery {
    pub(crate) data_source: String,
}

#[derive(Debug, Identifiable, Insertable, Queryable, QueryableByName, Serialize)]
#[table_name = "data_source"]
pub(crate) struct DataSourceTable {
    pub(crate) id: i32,
    pub(crate) topic_name: String,
    pub(crate) data_type: String,
}

pub(crate) fn add_data_source(pool: &web::Data<Pool>, data_source: &str, data_type: &str) -> i32 {
    use data_source::dsl;

    let new_data_source: Result<DataSourceTable, Error> =
        pool.get().map_err(Into::into).and_then(|conn| {
            diesel::insert_into(dsl::data_source)
                .values((
                    dsl::topic_name.eq(data_source),
                    dsl::data_type.eq(data_type),
                ))
                .on_conflict(dsl::topic_name)
                .do_nothing()
                .get_result(&conn)
                .map_err(Into::into)
        });

    new_data_source.ok().map_or(0, |d| d.id)
}

pub(crate) fn get_data_source_id(pool: &web::Data<Pool>, data_source: &str) -> Result<i32, Error> {
    use data_source::dsl;
    pool.get().map_err(Into::into).and_then(|conn| {
        dsl::data_source
            .select(dsl::id)
            .filter(dsl::topic_name.eq(data_source))
            .first::<i32>(&conn)
            .map_err(Into::into)
    })
}
