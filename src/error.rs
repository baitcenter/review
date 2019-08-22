use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::fmt::Display;

#[derive(Clone, Debug, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "REview could not start up successfully: {}", _0)]
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
    ClientMode,
    ClientUrl,
    DatabaseFileNotFound,
    DatabaseInitialization,
    MissingDatabaseURL,
    MissingDockerHostIp,
    MissingEtcdAddr,
    MissingKafkaUrl,
    MissingReviewdAddr,
    REviewd,
    REviewdUrl,
}

impl Display for InitializeErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InitializeErrorReason::ClientMode => write!(
                f,
                "Could not create the cluster view for REview client mode"
            ),
            InitializeErrorReason::ClientUrl => write!(
                f,
                "Invalid url format. Please specify a url in the form of http://<hostname>:<port number>"
            ),
            InitializeErrorReason::DatabaseFileNotFound => write!(f, "Could not find the database file"),
            InitializeErrorReason::DatabaseInitialization => write!(f, "Could not initialize the database"),
            InitializeErrorReason::MissingDatabaseURL => write!(f, "DATABASE_URL is not set"),
            InitializeErrorReason::MissingEtcdAddr => write!(f, "ETCD_ADDR is not set"),
            InitializeErrorReason::MissingReviewdAddr => write!(f, "REVIEWD_ADDR is not set"),
            InitializeErrorReason::MissingDockerHostIp => write!(f, "DOCKER_HOST_IP is not set"),
            InitializeErrorReason::MissingKafkaUrl => write!(f, "KAFKA_URL is not set"),                                    
            InitializeErrorReason::REviewd => write!(f, "Could not initialize REviewd"),
            InitializeErrorReason::REviewdUrl => write!(
                f,
                "IP address and/or port number for reviewd is bad/illegal format"
            ),
        }
    }
}
