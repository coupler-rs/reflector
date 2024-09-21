use graphics::{Affine, Canvas, Color, Path};

use super::{Context, Elem, Event, Response};
use crate::{AsAny, Build, Point, ProposedSize, Size};

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
    type Elem = ButtonElem;

    fn build(self) -> Self::Elem {
        ButtonElem {
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

trait Action: FnMut() + AsAny {}

impl<T: FnMut() + AsAny> Action for T {}

pub struct ButtonElem {
    label: Box<dyn Elem>,
    action: Box<dyn Action>,
    size: Size,
    hover: bool,
}

impl Elem for ButtonElem {
    fn update(&mut self, cx: &mut Context) {
        self.label.update(cx);
    }

    fn hit_test(&mut self, _cx: &mut Context, point: Point) -> bool {
        point.x >= 0.0 && point.x < self.size.width && point.y >= 0.0 && point.y < self.size.height
    }

    fn handle(&mut self, _cx: &mut Context, event: &Event) -> Response {
        match event {
            Event::MouseEnter => {
                self.hover = true;
            }
            Event::MouseExit => {
                self.hover = false;
            }
            Event::MouseDown(_) => {
                (self.action)();
                return Response::Capture;
            }
            _ => {}
        }

        Response::Ignore
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
        self.label.measure(cx, proposal)
    }

    fn place(&mut self, cx: &mut Context, size: Size) {
        self.size = size;
        self.label.place(cx, size);
    }

    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        let mut rect = Path::new();
        rect.move_to(Point::new(0.0, 0.0));
        rect.line_to(Point::new(0.0, self.size.height));
        rect.line_to(Point::new(self.size.width, self.size.height));
        rect.line_to(Point::new(self.size.width, 0.0));
        rect.close();

        if self.hover {
            canvas.fill_path(&rect, Affine::id(), Color::rgba(220, 220, 220, 255));
        } else {
            canvas.fill_path(&rect, Affine::id(), Color::rgba(180, 180, 180, 255));
        }

        self.label.render(cx, canvas);
    }
}
