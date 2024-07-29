#[doc(inline)]
pub extern crate reflector_graphics as graphics;
#[doc(inline)]
pub extern crate reflector_platform as platform;

pub use platform::{Point, Size};

mod app;
mod elem;
mod error;
mod window;

pub mod elems;

pub use app::App;
pub use elem::{Build, Child, Constraints, Context, Elem, Event, Response};
pub use error::{Error, Result};
pub use window::{Window, WindowOptions};
