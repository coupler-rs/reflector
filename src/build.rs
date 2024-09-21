pub use crate::elem::Elem;

pub trait Build {
    type Elem: Elem;

    fn build(self) -> Self::Elem;
    fn rebuild(self, elem: &mut Self::Elem);
}
