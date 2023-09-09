use std::fmt;

use x11rb::errors::{ConnectError, ConnectionError, ReplyError, ReplyOrIdError};

use crate::Error;

#[derive(Debug)]
pub enum OsError {
    Connect(ConnectError),
    Connection(ConnectionError),
    Reply(ReplyError),
    ReplyOrId(ReplyOrIdError),
    Message(&'static str),
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsError::Connect(err) => err.fmt(fmt),
            OsError::Connection(err) => err.fmt(fmt),
            OsError::Reply(err) => err.fmt(fmt),
            OsError::ReplyOrId(err) => err.fmt(fmt),
            OsError::Message(message) => write!(fmt, "{}", message),
        }
    }
}

impl From<ConnectError> for Error {
    fn from(err: ConnectError) -> Error {
        Error::Os(OsError::Connect(err))
    }
}

impl From<ConnectionError> for Error {
    fn from(err: ConnectionError) -> Error {
        Error::Os(OsError::Connection(err))
    }
}

impl From<ReplyError> for Error {
    fn from(err: ReplyError) -> Error {
        Error::Os(OsError::Reply(err))
    }
}

impl From<ReplyOrIdError> for Error {
    fn from(err: ReplyOrIdError) -> Error {
        Error::Os(OsError::ReplyOrId(err))
    }
}
