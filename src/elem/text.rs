use graphics::{Affine, Canvas, Color, Font, TextLayout};

use super::{Context, Elem, Event, Response};
use crate::{Point, ProposedSize, Size};

pub struct Text {
    text: String,
    font: Font,
    size: f32,
    layout: TextLayout,
}

impl Text {
    pub fn new<T>(text: T, font: Font, size: f32) -> Text
    where
        T: AsRef<str>,
    {
        let text = text.as_ref().to_owned();
        let layout = TextLayout::new(&text, &font, size);

        Text {
            text,
            font,
            size,
            layout,
        }
    }

    pub fn set_text<T>(&mut self, text: T)
    where
        T: AsRef<str>,
    {
        self.text.clear();
        self.text.push_str(text.as_ref());
        self.layout = TextLayout::new(&self.text, &self.font, self.size);
    }

    pub fn set_font(&mut self, font: Font) {
        self.font = font;
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }
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
