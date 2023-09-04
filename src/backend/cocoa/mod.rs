use std::fmt;

mod app;
mod ffi;
mod surface;
mod window;

pub use app::{AppContextInner, AppInner};
pub use window::WindowInner;

#[derive(Debug)]
pub enum OsError {
    Other(&'static str),
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsError::Other(err) => write!(fmt, "{}", err),
        }
    }
}

pub struct TimerHandleInner {}

impl TimerHandleInner {
    pub fn cancel(self) {}
}
