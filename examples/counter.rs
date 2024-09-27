use std::cell::Cell;
use std::rc::Rc;

use reflector::build::{Build, Button, Row, Text};
use reflector::elem::{Context, Elem, Event, Response};
use reflector::graphics::{Canvas, Font};
use reflector::{App, Point, ProposedSize, Size, WindowOptions};

struct Counter {
    counter: Rc<Cell<usize>>,
    inner: Box<dyn Elem>,
}

impl Counter {
    fn new() -> Counter {
        let counter = Rc::new(Cell::new(0));
        let inner = Box::new(Self::build(&counter).build());

        Counter { counter, inner }
    }

    fn build(counter: &Rc<Cell<usize>>) -> impl Build {
        let font = Font::from_bytes(
            include_bytes!("../reflector-graphics/examples/res/SourceSansPro-Regular.otf"),
            0,
        )
        .unwrap();

        let counter = counter.clone();

        Row::new(5.0)
            .child(Text::new(format!("{}", counter.get()), font.clone(), 16.0))
            .child(
                Button::new(Text::new("button", font.clone(), 16.0))
                    .action(move || counter.set(counter.get() + 1)),
            )
    }
}

impl Elem for Counter {
    fn update(&mut self, cx: &mut Context) {
        Self::build(&self.counter).rebuild(self.inner.downcast_mut().unwrap());

        self.inner.update(cx);
    }

    fn hit_test(&mut self, cx: &mut Context, point: Point) -> bool {
        self.inner.hit_test(cx, point)
    }

    fn handle(&mut self, cx: &mut Context, event: &Event) -> Response {
        self.inner.handle(cx, event)
    }

    fn measure(&mut self, cx: &mut Context, proposal: ProposedSize) -> Size {
        self.inner.measure(cx, proposal)
    }

    fn place(&mut self, cx: &mut Context, size: Size) {
        self.inner.place(cx, size);
    }

    fn render(&mut self, cx: &mut Context, canvas: &mut Canvas) {
        self.inner.render(cx, canvas);
    }
}

fn main() {
    let app = App::new().unwrap();

    WindowOptions::new()
        .title("window")
        .size(Size::new(512.0, 512.0))
        .open(&app, Counter::new())
        .unwrap();

    app.run().unwrap();
}
