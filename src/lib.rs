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
pub mod list;

pub use app::{App, AppOptions};
pub use elem::{AsAny, BuildElem, Elem, ElemContext, ElemEvent, Response};
pub use error::{Error, Result};
pub use geom::{Point, ProposedSize, Size};
pub use window::{Window, WindowOptions};
