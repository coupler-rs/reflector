use std::{error, fmt, result};

use reflector_platform as platform;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Platform(platform::Error),
}

impl From<platform::Error> for Error {
    fn from(err: platform::Error) -> Error {
        Error::Platform(err)
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Platform(err) => err.fmt(fmt),
        }
    }
}
