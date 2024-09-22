use graphics::{Affine, Canvas, Color, Path};

use super::{Context, Elem, Event, Response};
use crate::{AsAny, Point, ProposedSize, Size};

trait Action: FnMut() + AsAny {}

impl<T: FnMut() + AsAny> Action for T {}

pub struct Button {
    label: Box<dyn Elem>,
    action: Box<dyn Action>,
    size: Size,
    hover: bool,
}

impl Button {
    pub fn new<E>(label: E) -> Button
    where
        E: Elem,
    {
        Button {
            label: Box::new(label),
            action: Box::new(|| {}),
            size: Size::new(0.0, 0.0),
            hover: false,
        }
    }

    pub fn set_label<E>(&mut self, label: E)
    where
        E: Elem,
    {
        self.label = Box::new(label);
    }

    pub fn label_mut(&mut self) -> &mut dyn Elem {
        &mut *self.label
    }

    pub fn set_action<F>(&mut self, action: F)
    where
        F: FnMut() + 'static,
    {
        if let Some(old_action) = self.action.as_mut_any().downcast_mut() {
            *old_action = action;
        } else {
            self.action = Box::new(action);
        }
    }
}

impl Elem for Button {
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
