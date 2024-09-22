pub use crate::elem::Elem;

mod button;
mod padding;
mod row;
mod text;

pub use button::Button;
pub use padding::Padding;
pub use row::Row;
pub use text::Text;

pub trait Build {
    type Elem: Elem;

    fn build(self) -> Self::Elem;
    fn rebuild(self, elem: &mut Self::Elem);
}
