use graphics::{Affine, Canvas};

use super::{Context, Elem, Event, Response};
use crate::{Build, Point, ProposedSize, Size};

pub struct Padding<E> {
    padding_x: f32,
    padding_y: f32,
    child: E,
}

impl<E: Build> Padding<E> {
    pub fn new(padding: f32, child: E) -> Padding<E> {
        Padding {
            padding_x: padding,
            padding_y: padding,
            child,
        }
    }

    pub fn new_xy(padding_x: f32, padding_y: f32, child: E) -> Padding<E> {
        Padding {
            padding_x,
            padding_y,
            child,
        }
    }
}

impl<E: Build> Build for Padding<E> {
    type Elem = PaddingElem;

    fn build(self) -> Self::Elem {
        PaddingElem {
            padding_x: self.padding_x,
            padding_y: self.padding_y,
            child: Box::new(self.child.build()),
        }
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        elem.padding_x = self.padding_x;
        elem.padding_y = self.padding_y;

        if let Some(child) = elem.child.downcast_mut() {
            self.child.rebuild(child);
        } else {
            elem.child = Box::new(self.child.build());
        }
    }
}

pub struct PaddingElem {
    padding_x: f32,
    padding_y: f32,
    child: Box<dyn Elem>,
}

impl Elem for PaddingElem {
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
