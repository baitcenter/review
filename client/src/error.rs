use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::fmt::Display;

#[derive(Clone, Debug, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(
        display = "REview http client mode could not start up successfully: {}",
        _0
    )]
    Initialize(InitializeErrorReason),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InitializeErrorReason {
    UnexpectedResponse,
    Reqwest,
    EmptyCluster,
}

impl Display for InitializeErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InitializeErrorReason::UnexpectedResponse => write!(
                f,
                "Unexpected response from server. Cannot deserialize the received data from REviewd"
            ),
            InitializeErrorReason::Reqwest => write!(
                f,
                "An error occurs while sending an http request to REviewd"
            ),
            InitializeErrorReason::EmptyCluster => {
                write!(f, "Cluster with pending review status was not found")
            }
        }
    }
}
