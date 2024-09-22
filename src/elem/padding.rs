use graphics::{Affine, Canvas};

use super::{Context, Elem, Event, Response};
use crate::{Point, ProposedSize, Size};

pub struct Padding {
    padding_x: f32,
    padding_y: f32,
    child: Box<dyn Elem>,
}

impl Padding {
    pub fn new<E>(padding: f32, child: E) -> Padding
    where
        E: Elem,
    {
        Self::new_xy(padding, padding, child)
    }

    pub fn new_xy<E>(padding_x: f32, padding_y: f32, child: E) -> Padding
    where
        E: Elem,
    {
        Padding {
            padding_x,
            padding_y,
            child: Box::new(child),
        }
    }

    pub fn set_padding(&mut self, padding: f32) {
        self.padding_x = padding;
        self.padding_y = padding;
    }

    pub fn set_padding_xy(&mut self, padding_x: f32, padding_y: f32) {
        self.padding_x = padding_x;
        self.padding_y = padding_y;
    }

    pub fn set_child<E>(&mut self, child: E)
    where
        E: Elem,
    {
        self.child = Box::new(child);
    }

    pub fn child_mut(&mut self) -> &mut dyn Elem {
        &mut *self.child
    }
}

impl Elem for Padding {
    fn update(&mut self, cx: &mut Context) {
        self.child.update(cx);
    }

    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool {
        self.child.hit_test(cx, point - Point::new(self.padding_x, self.padding_y))
    }

    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response {
        self.child.handle(cx, event)
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
        let proposal = proposal.shrink(2.0 * self.padding_x, 2.0 * self.padding_y);
        let size = self.child.measure(cx, proposal);
        size.grow(2.0 * self.padding_x, 2.0 * self.padding_y)
    }

    fn place(&mut self, cx: &mut Context, size: Size) {
        self.child.place(cx, size.shrink(2.0 * self.padding_x, 2.0 * self.padding_y));
    }

    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        let transform = Affine::translate(self.padding_x, self.padding_y);
        canvas.with_transform(transform, |canvas| {
            self.child.render(cx, canvas);
        });
    }
}
