use graphics::{Affine, Canvas, Color, Font, TextLayout};

use crate::{Build, Context, Elem, Event, ProposedSize, Response, Size};

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
        TextElem {
            text: self.text.as_ref().to_owned(),
            font: self.font,
            size: self.size,
            layout: TextLayout::empty(),
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

    fn event(&mut self, _cx: &mut Context, _event: Event) -> Response {
        Response::Ignore
    }

    fn layout(&mut self, _cx: &mut Context, proposal: ProposedSize) -> Size {
        self.layout = TextLayout::new(&self.text, &self.font, self.size);

        proposal.unwrap_or(Size::new(0.0, 0.0))
    }

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
