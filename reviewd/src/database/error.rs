use diesel::result::Error as QueryResultError;
use diesel::ConnectionError;
use diesel_migrations::RunMigrationsError;
use failure::{Backtrace, Context, Fail};
use r2d2::Error as R2D2Error;
use serde_json::error::Error as JsonDeserializationError;
use std::fmt;
use std::fmt::Display;

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(
        display = "An error occurred while processing a database transaction : {}",
        _0
    )]
    DatabaseTransactionError(DatabaseError),
    #[fail(display = "diesel connection error")]
    Connection,
    #[fail(display = "A migration error")]
    Migration,
    #[fail(display = "An error occurred while processing a database query")]
    Query,
    #[fail(display = "An error occurred while connecting database")]
    R2D2,
    #[fail(display = "An error occurred while deserializing data in JSON format")]
    SerdeJson,
}

impl From<ConnectionError> for Error {
    fn from(error: ConnectionError) -> Self {
        Self {
            inner: error.context(ErrorKind::Connection),
        }
    }
}

impl From<QueryResultError> for Error {
    fn from(error: QueryResultError) -> Self {
        Self {
            inner: error.context(ErrorKind::Query),
        }
    }
}

impl From<R2D2Error> for Error {
    fn from(error: R2D2Error) -> Self {
        Self {
            inner: error.context(ErrorKind::R2D2),
        }
    }
}

impl From<RunMigrationsError> for Error {
    fn from(error: RunMigrationsError) -> Self {
        Self {
            inner: error.context(ErrorKind::Migration),
        }
    }
}

impl From<JsonDeserializationError> for Error {
    fn from(error: JsonDeserializationError) -> Self {
        Self {
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
/*
impl Error {
    pub fn new(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }

    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}
*/
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Self { inner }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DatabaseError {
    RecordNotExist,
    Other,
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordNotExist => write!(f, "Record does not exist in database"),
            Self::Other => write!(f, "An error occurred"),
        }
    }
}
