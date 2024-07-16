use reflector::{
    App, Build, Canvas, Color, Constraints, Context, Elem, Event, Response, Size, WindowOptions,
};

struct Root {}

impl Build for Root {
    type Result = RootElem;

    fn build(self, _cx: &mut Context) -> RootElem {
        RootElem {}
    }

    fn rebuild(self, _cx: &mut Context, _result: &mut RootElem) {}
}

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
        .open(&app, Root {})
        .unwrap();

    app.run().unwrap();
}
