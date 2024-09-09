use std::any::Any;

#[doc(inline)]
pub extern crate reflector_graphics as graphics;
#[doc(inline)]
pub extern crate reflector_platform as platform;

mod app;
mod elem;
mod error;
mod geom;
mod proc;
mod window;

pub mod elems;
pub mod list;

pub use app::{App, AppOptions};
pub use elem::{BuildElem, Elem, ElemContext, ElemEvent};
pub use error::{Error, Result};
pub use geom::{Point, ProposedSize, Size};
pub use proc::{BuildProc, Proc, ProcContext, ProcEvent};
pub use window::{Window, WindowOptions};

#[derive(PartialEq, Eq, Debug)]
pub enum Response {
    Capture,
    Ignore,
}

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}
