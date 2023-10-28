use std::{error, fmt, result};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Os(crate::backend::OsError),
    WindowClosed,
    InsideEventHandler,
    InvalidWindowHandle,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Os(err) => write!(fmt, "os error: {}", err),
            Error::WindowClosed => write!(fmt, "window closed"),
            Error::InsideEventHandler => {
                write!(fmt, "operation not supported inside an event handler")
            }
            Error::InvalidWindowHandle => write!(fmt, "invalid window handle"),
        }
    }
}

#[derive(Debug)]
pub struct IntoInnerError<T> {
    error: Error,
    inner: T,
}

impl<T> IntoInnerError<T> {
    #[cfg_attr(target_os = "linux", allow(unused))]
    pub(crate) fn new(error: Error, inner: T) -> IntoInnerError<T> {
        IntoInnerError { error, inner }
    }

    #[inline]
    pub fn error(&self) -> &Error {
        &self.error
    }

    #[inline]
    pub fn into_error(self) -> Error {
        self.error
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[inline]
    pub fn into_parts(self) -> (Error, T) {
        (self.error, self.inner)
    }
}

impl<T: Send + fmt::Debug> error::Error for IntoInnerError<T> {}

impl<T> fmt::Display for IntoInnerError<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(fmt)
    }
}
