use std::any::Any;

use graphics::Canvas;
pub use platform::MouseButton;

use crate::{AsAny, Point, ProposedSize, Size};

mod button;
mod padding;
mod row;
mod text;

pub use button::Button;
pub use padding::Padding;
pub use row::Row;
pub use text::Text;

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
