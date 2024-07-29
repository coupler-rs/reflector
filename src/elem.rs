use graphics::Canvas;

use crate::Size;

pub struct Context {}

pub struct Constraints {
    pub min: Size,
    pub max: Size,
}

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
    fn layout(&mut self, cx: &mut Context, constraints: Constraints) -> Size;
    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas);
}

pub struct Child<E> {
    elem: E,
}

impl<E: Elem> Child<E> {
    pub fn new(elem: E) -> Child<E> {
        Child { elem }
    }

    pub fn rebuild<B>(&mut self, cx: &mut Context, builder: B)
    where
        B: Build<Elem = E>,
    {
        builder.rebuild(cx, &mut self.elem);
    }

    pub fn update(&mut self, cx: &mut Context) {
        self.elem.update(cx);
    }

    pub fn event(&mut self, cx: &mut Context, event: Event) -> Response {
        self.elem.event(cx, event)
    }

    pub fn layout(&mut self, cx: &mut Context, constraints: Constraints) -> Size {
        self.elem.layout(cx, constraints)
    }

    pub fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        self.elem.render(cx, canvas);
    }
}
