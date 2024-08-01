use graphics::Canvas;

use crate::{ProposedSize, Size};

pub struct Context {}

pub enum Event {}

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
    fn event(&mut self, cx: &mut Context, event: Event) -> Response;
    fn layout(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size;
    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas);
}
