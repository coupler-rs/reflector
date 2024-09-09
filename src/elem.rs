use std::any::Any;

use graphics::Canvas;
pub use platform::MouseButton;

use crate::{AsAny, Point, ProposedSize, Response, Size};

pub struct ElemContext {}

#[derive(Clone, Debug)]
pub enum ElemEvent {
    MouseEnter,
    MouseExit,
    MouseMove(Point),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    Scroll(Point),
}

pub trait BuildElem {
    type Elem: Elem;

    fn build(self, cx: &mut ElemContext) -> Self::Elem;
    fn rebuild(self, cx: &mut ElemContext, elem: &mut Self::Elem);
}

pub trait Elem: AsAny {
    fn update(&mut self, cx: &mut ElemContext);
    fn hit_test(&mut self, cx: &mut ElemContext, point: Point) -> bool;
    fn handle(&mut self, cx: &mut ElemContext, event: &ElemEvent) -> Response;
    fn measure(&mut self, cx: &mut ElemContext, proposal: ProposedSize) -> Size;
    fn place(&mut self, cx: &mut ElemContext, size: Size);
    fn render(&mut self, cx: &mut ElemContext, canvas: &mut Canvas);
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
