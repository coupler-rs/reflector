use std::fmt;

mod app;
mod window;

pub use app::{AppContextInner, AppInner};
pub use window::WindowInner;

#[derive(Debug)]
pub struct OsError {}

impl fmt::Display for OsError {
    fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

pub struct TimerHandleInner {}

impl TimerHandleInner {
    pub fn cancel(self) {}
}
