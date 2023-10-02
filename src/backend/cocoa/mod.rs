use std::fmt;

mod app;
mod display_links;
mod ffi;
mod surface;
mod timer;
mod window;

pub use app::{AppContextInner, AppInner};
pub use timer::TimerHandleInner;
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
