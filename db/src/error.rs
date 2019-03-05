use diesel::result::Error as QueryResultError;
use failure::{Backtrace, Context, Fail};
use r2d2::Error as R2D2Error;
use std::fmt;
use std::fmt::Display;

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "An error occurred while processing a database query")]
    Query,
    #[fail(display = "An error occurred while connecting database")]
    R2D2,
}

impl From<QueryResultError> for Error {
    fn from(error: QueryResultError) -> Error {
        Error {
            inner: error.context(ErrorKind::Query),
        }
    }
}

impl From<R2D2Error> for Error {
    fn from(error: R2D2Error) -> Error {
        Error {
            inner: error.context(ErrorKind::R2D2),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
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