#[doc(inline)]
pub extern crate reflector_graphics as graphics;
#[doc(inline)]
pub extern crate reflector_platform as platform;

mod app;
mod elem;
mod error;
mod geom;
mod window;

pub mod elems;

pub use app::App;
pub use elem::{Build, Child, Constraints, Context, Elem, Event, Response};
pub use error::{Error, Result};
pub use geom::{Point, Size};
pub use window::{Window, WindowOptions};
