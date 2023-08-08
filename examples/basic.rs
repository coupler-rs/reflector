use std::time::Duration;

use portlight::{App, AppContext, Bitmap, Event, Response, Size, Window, WindowOptions};

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
            Event::Expose(rects) => {
                println!("expose: {:?}", rects);
                self.window.present(Bitmap::new(&[0xFFFF00FF; 512 * 512], 512, 512));
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
            Event::Close => {
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

        cx.set_timer(Duration::from_millis(1000), |_, _| {
            println!("timer");
        });

        Ok(State { window: window })
    })
    .unwrap()
    .run()
    .unwrap();
}
