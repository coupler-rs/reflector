use portlight::{App, AppContext, Event, Response, Size, Window, WindowOptions};

struct State {
    window: Window,
}

impl Drop for State {
    fn drop(&mut self) {
        println!("drop");
    }
}

impl State {
    fn handle_event(&mut self, cx: &AppContext<Self>, event: Event) -> Response {
        match event {
            Event::Frame => {
                println!("frame");
                self.window.request_display();
            }
            Event::Display => {
                println!("display");
                self.window
                    .update_contents(&[0xFFFF00FF; 512 * 512], 512, 512);
            }
            Event::MouseMove(pos) => {
                println!("mouse move: {:?}", pos);
            }
            Event::MouseDown(btn) => {
                println!("mouse down: {:?}", btn);
                return Response::Capture;
            }
            Event::MouseUp(btn) => {
                println!("mouse up: {:?}", btn);
                return Response::Capture;
            }
            Event::Scroll(delta) => {
                println!("scroll: {:?}", delta);
                return Response::Capture;
            }
            Event::RequestClose => {
                cx.exit();
            }
        }

        Response::Ignore
    }
}

fn main() {
    App::new(|cx| {
        let window = WindowOptions::new()
            .title("window")
            .size(Size::new(512.0, 512.0))
            .open(cx, State::handle_event)
            .unwrap();

        window.show();

        Ok(State { window: window })
    })
    .unwrap()
    .run()
    .unwrap();
}
