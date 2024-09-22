#[doc(inline)]
pub extern crate reflector_graphics as graphics;
#[doc(inline)]
pub extern crate reflector_platform as platform;

mod any;
mod app;
mod error;
mod geom;
mod window;

pub mod build;
pub mod elem;
pub mod list;

pub use any::AsAny;
pub use app::{App, AppOptions};
pub use error::{Error, Result};
pub use geom::{Point, ProposedSize, Size};
pub use window::{Window, WindowOptions};
