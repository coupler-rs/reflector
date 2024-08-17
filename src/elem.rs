use std::any::Any;

use graphics::Canvas;
pub use platform::MouseButton;

use crate::{Point, ProposedSize, Size};

pub struct Context {}

#[derive(Clone, Debug)]
pub enum Event {
    MouseEnter,
    MouseExit,
    MouseMove(Point),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    Scroll(Point),
}

#[derive(PartialEq, Eq, Debug)]
pub enum Response {
    Capture,
    Ignore,
}

pub trait Build {
    type Elem: Elem;

    fn build(self, cx: &mut Context) -> Self::Elem;
    fn rebuild(self, cx: &mut Context, elem: &mut Self::Elem);
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

pub trait Elem: AsAny {
    fn update(&mut self, cx: &mut Context);
    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool;
    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response;
    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size;
    fn place(&mut self, cx: &mut Context, size: Size);
    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas);
}

impl dyn Elem {
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        self.as_mut_any().downcast_mut()
    }
}
