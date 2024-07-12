extern crate reflector_platform as platform;

pub use platform::{Point, Size};

mod app;
mod error;
mod window;

pub use app::App;
pub use error::{Error, Result};
pub use window::{Window, WindowOptions};
