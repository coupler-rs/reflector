use super::Build;
use crate::elem;

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
        let mut button = elem::Button::new(self.label.build());
        button.set_action(self.action);
        button
    }

    fn rebuild(self, elem: &mut Self::Elem) {
        if let Some(label) = elem.label_mut().downcast_mut() {
            self.label.rebuild(label);
        } else {
            elem.set_label(self.label.build());
        }

        elem.set_action(self.action);
    }
}
