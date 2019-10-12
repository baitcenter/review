use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use failure::Fail;
use serde_json::json;

mod category;
mod cluster;
mod data_source;
mod error;
mod outlier;
mod qualifier;
mod raw_event;
mod schema;
mod status;

pub(crate) use self::category::*;
pub(crate) use self::cluster::*;
pub(crate) use self::data_source::*;
pub(crate) use self::error::{DatabaseError, Error, ErrorKind};
pub(crate) use self::outlier::*;
pub(crate) use self::qualifier::*;
pub(crate) use self::raw_event::*;
pub(crate) use self::status::*;

// for client crate
pub use self::cluster::{Example, QualifierUpdate};
pub use self::qualifier::QualifierTable;

pub(crate) type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub(crate) fn build_err_msg(fail: &dyn Fail) -> String {
    let mut err_msg = fail.to_string();
    for cause in fail.iter_causes() {
        err_msg.push_str(&format!("\n\tcaused by: {}", cause));
    }

    json!({"message": err_msg,
    })
    .to_string()
}
