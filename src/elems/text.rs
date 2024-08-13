use graphics::{Affine, Canvas, Color, Font, TextLayout};

use crate::{Build, Context, Elem, Event, ProposedSize, Response, Size, Point};

pub struct Text<T> {
    text: T,
    font: Font,
    size: f32,
}

impl<T> Text<T>
where
    T: AsRef<str>,
{
    pub fn new(text: T, font: Font, size: f32) -> Text<T> {
        Text { text, font, size }
    }
}

impl<T> Build for Text<T>
where
    T: AsRef<str>,
{
    type Elem = TextElem;

    fn build(self, _cx: &mut Context) -> Self::Elem {
        let text = self.text.as_ref().to_owned();
        let layout = TextLayout::new(&text, &self.font, self.size);

        TextElem {
            text,
            font: self.font,
            size: self.size,
            layout,
        }
    }

    fn rebuild(self, _cx: &mut Context, elem: &mut Self::Elem) {
        elem.text.clear();
        elem.text.push_str(self.text.as_ref());
        elem.font = self.font;
        elem.size = self.size;
    }
}

pub struct TextElem {
    text: String,
    font: Font,
    size: f32,
    layout: TextLayout,
}

impl Elem for TextElem {
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
