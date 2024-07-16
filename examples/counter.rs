use reflector::{
    App, Canvas, Color, Constraints, Context, Elem, Event, Response, Size, WindowOptions,
};

struct RootElem {}

impl Elem for RootElem {
    fn update(&mut self, _cx: &mut Context) {}

    fn event(&mut self, _cx: &mut Context, _event: Event) -> Response {
        Response::Ignore
    }

    fn layout(&mut self, _cx: &mut Context, constraints: Constraints) -> Size {
        constraints.max
    }

    fn render(&mut self, _cx: &mut Context, canvas: &mut Canvas) {
        canvas.clear(Color::rgba(255, 255, 255, 255));
    }
}

fn main() {
    let app = App::new().unwrap();

    WindowOptions::new()
        .title("window")
        .size(Size::new(512.0, 512.0))
        .open(&app, RootElem {})
        .unwrap();

    app.run().unwrap();
}
