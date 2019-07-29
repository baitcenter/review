use failure::Fail;
use log::{log, Level};

pub mod app;
mod error;

pub fn log_error(fail: &Fail) {
    log!(Level::Error, "{}", fail);
    for cause in fail.iter_causes() {
        log!(Level::Error, "\tcaused by: {}", cause);
    }
}
