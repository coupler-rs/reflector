mod app;
mod window;

use std::fmt;
use std::os::raw::c_int;

pub use app::{AppContextInner, AppInner};
pub use window::WindowInner;

#[derive(Debug)]
pub enum OsError {
    Xcb(c_int),
    Message(&'static str),
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsError::Xcb(code) => write!(fmt, "{}", code),
            OsError::Message(message) => write!(fmt, "{}", message),
        }
    }
}

pub struct TimerHandleInner {}

impl TimerHandleInner {
    pub fn cancel(self) {}
}
