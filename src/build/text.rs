use graphics::{Font, TextLayout};

use super::Build;
use crate::elem;

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
    type Elem = elem::Text;

    fn build(self) -> Self::Elem {
        let text = self.text.as_ref().to_owned();
        let layout = TextLayout::new(&text, &self.font, self.size);

        elem::Text {
            text,
            font: self.font,
            size: self.size,
            layout,
        }
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        elem.text.clear();
        elem.text.push_str(self.text.as_ref());
        elem.font = self.font;
        elem.size = self.size;
    }
}
