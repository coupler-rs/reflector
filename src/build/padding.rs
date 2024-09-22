use super::Build;
use crate::elem;

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
    type Elem = elem::Padding;

    fn build(self) -> Self::Elem {
        elem::Padding::new_xy(self.padding_x, self.padding_y, self.child.build())
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        elem.set_padding_xy(self.padding_x, self.padding_y);

        if let Some(child) = elem.child_mut().downcast_mut() {
            self.child.rebuild(child);
        } else {
            elem.set_child(self.child.build());
        }
    }
}
