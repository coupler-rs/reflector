use super::Build;
use crate::elem;
use crate::Size;

pub struct Button<E, F> {
    label: E,
    action: F,
}

impl<E> Button<E, ()> {
    pub fn new(label: E) -> Button<E, impl FnMut()> {
        Button {
            label,
            action: || {},
        }
    }
}

impl<E, F> Button<E, F> {
    pub fn action<G: FnMut()>(self, action: G) -> Button<E, G> {
        Button {
            label: self.label,
            action,
        }
    }
}

impl<E, F> Build for Button<E, F>
where
    E: Build,
    F: FnMut() + 'static,
{
    type Elem = elem::Button;

    fn build(self) -> Self::Elem {
        elem::Button {
            label: Box::new(self.label.build()),
            action: Box::new(self.action),
            size: Size::new(0.0, 0.0),
            hover: false,
        }
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        if let Some(label) = elem.label.downcast_mut() {
            self.label.rebuild(label);
        } else {
            elem.label = Box::new(self.label.build());
        }

        if let Some(action) = elem.action.as_mut_any().downcast_mut() {
            *action = self.action;
        } else {
            elem.action = Box::new(self.action);
        }
    }
}
