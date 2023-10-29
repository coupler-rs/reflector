use std::time::Duration;

use portlight::{AppContext, AppOptions, Bitmap, Event, Response, Size, Window, WindowOptions};

const WIDTH: usize = 512;
const HEIGHT: usize = 512;

struct State {
    window: Window,
    framebuffer: Vec<u32>,
    width: usize,
    height: usize,
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
                self.window.present(Bitmap::new(&self.framebuffer, self.width, self.height));
            }
            Event::Frame => {
                println!("frame");
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
    let app = AppOptions::new()
        .build(|cx| {
            let window = WindowOptions::new()
                .title("window")
                .size(Size::new(512.0, 512.0))
                .open(cx, State::handle_event)
                .unwrap();

            cx.set_timer(Duration::from_millis(1000), |_, _| {
                println!("timer");
            });

            let scale = window.scale();
            let width = (WIDTH as f64 * scale) as usize;
            let height = (HEIGHT as f64 * scale) as usize;
            let framebuffer = vec![0xFFFF00FF; width * height];
            window.present(Bitmap::new(&framebuffer, width, height));

            window.show();

            Ok(State {
                window,
                framebuffer,
                width,
                height,
            })
        })
        .unwrap();

    app.run().unwrap();
}
