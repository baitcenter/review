use db::Error as DBError;
use failure::{Backtrace, Context, Fail};
use hyper::Error as HyperError;
use serde_json::error::Error as JsonDeserializationError;
use std::fmt;
use std::fmt::Display;

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "An error occurred on HTTP server")]
    Hyper,
    #[fail(display = "An error occurred on database")]
    DB,
    #[fail(display = "An error occurred while deserializing data in JSON format")]
    SerdeJson,
}

impl From<DBError> for Error {
    fn from(error: DBError) -> Error {
        Error {
            inner: error.context(ErrorKind::DB),
        }
    }
}

impl From<HyperError> for Error {
    fn from(error: HyperError) -> Error {
        Error {
            inner: error.context(ErrorKind::Hyper),
        }
    }
}

impl From<JsonDeserializationError> for Error {
    fn from(error: JsonDeserializationError) -> Error {
        Error {
            inner: error.context(ErrorKind::SerdeJson),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn new(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }

    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}
