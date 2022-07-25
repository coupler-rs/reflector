use portlight::{App, AppContext, Event, Response, Size, Window, WindowOptions};

struct State {
    _window: Window,
}

impl Drop for State {
    fn drop(&mut self) {
        println!("drop");
    }
}

impl State {
    fn handle_event(&mut self, cx: &AppContext<Self>, event: Event) -> Response {
        match event {
            Event::RequestClose => {
                cx.exit();
                Response::Capture
            }
            _ => Response::Ignore,
        }
    }
}

fn main() {
    App::new(|cx| {
        let window = WindowOptions::new()
            .title("window")
            .size(Size::new(640.0, 480.0))
            .open(cx, State::handle_event)
            .unwrap();

        Ok(State { _window: window })
    })
    .unwrap()
    .run()
    .unwrap();
}
