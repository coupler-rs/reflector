use graphics::{Affine, Canvas, Color, Font, TextLayout};

use super::{Context, Elem, Event, Response};
use crate::{Point, ProposedSize, Size};

pub struct Text {
    pub(crate) text: String,
    pub(crate) font: Font,
    pub(crate) size: f32,
    pub(crate) layout: TextLayout,
}

impl Elem for Text {
    fn update(&mut self, _cx: &mut Context) {}

    fn hit_test(&mut self, _cx: &mut Context, point: Point) -> bool {
        point.x >= 0.0
            && point.x < self.layout.width()
            && point.y >= 0.0
            && point.y < self.layout.height()
    }

    fn handle(&mut self, _cx: &mut Context, _event: &Event) -> Response {
        Response::Ignore
    }

    fn measure(&mut self, _cx: &mut Context, _proposal: ProposedSize) -> Size {
        Size::new(self.layout.width(), self.layout.height())
    }

    fn place(&mut self, _cx: &mut Context, _size: Size) {}

    fn render(&mut self, _cx: &mut Context, canvas: &mut Canvas) {
        canvas.fill_glyphs(
            self.layout.glyphs(),
            &self.font,
            self.size,
            Affine::id(),
            Color::rgba(0, 0, 0, 255),
        );
    }
}
