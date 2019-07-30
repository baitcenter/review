use failure::Fail;
use log::error;

pub mod app;
mod error;

pub fn log_error(fail: &Fail) {
    error!("{}", fail);
    for cause in fail.iter_causes() {
        error!("\tcaused by: {}", cause);
    }
}
