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
    pub fn new(inner: Context<ErrorKind>) -> Self {
        Self { inner }
    }

    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

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
pub enum InitializeErrorReason {
    BuildServer,
    ClientMode,
    ClientUrl,
    MissingDatabaseURL,
    MissingDockerHostIp,
    MissingEtcdAddr,
    MissingKafkaUrl,
    MissingReviewdAddr,
    REviewd,
    REviewdUrl,
    ServerRun,
}

impl Display for InitializeErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BuildServer => write!(f, "Could not build server"),
            Self::ClientMode => write!(
                f,
                "Could not create the cluster view for REview client mode"
            ),
            Self::ClientUrl => write!(
                f,
                "Invalid url format. Please specify a url in the form of http://<hostname>:<port number>"
            ),
            Self::MissingDatabaseURL => write!(f, "DATABASE_URL is not set"),
            Self::MissingEtcdAddr => write!(f, "ETCD_ADDR is not set"),
            Self::MissingReviewdAddr => write!(f, "REVIEWD_ADDR is not set"),
            Self::MissingDockerHostIp => write!(f, "DOCKER_HOST_IP is not set"),
            Self::MissingKafkaUrl => write!(f, "KAFKA_URL is not set"),                                    
            Self::REviewd => write!(f, "Could not initialize REviewd"),
            Self::REviewdUrl => write!(
                f,
                "IP address and/or port number for reviewd is bad/illegal format"
            ),
            Self::ServerRun => write!(f, "Could not run server"),
        }
    }
}
