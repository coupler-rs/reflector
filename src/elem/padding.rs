use graphics::{Affine, Canvas};

use super::{Context, Elem, Event, Response};
use crate::{Point, ProposedSize, Size};

pub struct Padding {
    pub(crate) padding_x: f32,
    pub(crate) padding_y: f32,
    pub(crate) child: Box<dyn Elem>,
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
