use graphics::{Affine, Canvas};

use crate::{Build, Context, Elem, Event, ProposedSize, Response, Size};

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
    type Elem = PaddingElem<E::Elem>;

    fn build(self, cx: &mut Context) -> Self::Elem {
        PaddingElem {
            padding_x: self.padding_x,
            padding_y: self.padding_y,
            child: self.child.build(cx),
        }
    }

    fn rebuild(self, cx: &mut Context, elem: &mut Self::Elem) {
        elem.padding_x = self.padding_x;
        elem.padding_y = self.padding_y;
        self.child.rebuild(cx, &mut elem.child);
    }
}

pub struct PaddingElem<E> {
    padding_x: f32,
    padding_y: f32,
    child: E,
}

impl<E: Elem> Elem for PaddingElem<E> {
    fn update(&mut self, cx: &mut Context) {
        self.child.update(cx);
    }

    fn handle(&mut self, cx: &mut Context, event: Event) -> Response {
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
