use graphics::Canvas;
pub use platform::MouseButton;

use crate::{Point, ProposedSize, Size};

pub struct Context {}

#[derive(Clone, Debug)]
pub enum Event {
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

pub trait Elem {
    fn update(&mut self, cx: &mut Context);
    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool;
    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response;
    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size;
    fn place(&mut self, cx: &mut Context, size: Size);
    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas);
}
