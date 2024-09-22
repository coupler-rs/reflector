use graphics::Font;

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
        elem::Text::new(self.text, self.font, self.size)
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        elem.set_text(self.text);
        elem.set_font(self.font);
        elem.set_size(self.size);
    }
}
