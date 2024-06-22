use reflector_platform::{
    App, AppMode, AppOptions, Bitmap, Event, Point, Response, Size, WindowOptions,
};

fn main() {
    let parent_app = App::new().unwrap();

    let parent_window = WindowOptions::new()
        .title("parent window")
        .size(Size::new(512.0, 512.0))
        .open(parent_app.handle(), {
            let mut framebuffer = Vec::new();
            move |cx, event| {
                match event {
                    Event::Frame => {
                        let scale = cx.window().scale();
                        let size = cx.window().size();
                        let width = (scale * size.width) as usize;
                        let height = (scale * size.height) as usize;
                        framebuffer.resize(width * height, 0xFF00FFFF);
                        cx.window().present(Bitmap::new(&framebuffer, width, height));
                    }
                    Event::Close => {
                        cx.app().exit();
                    }
                    _ => {}
                }

                Response::Ignore
            }
        })
        .unwrap();

    let child_app = AppOptions::new().mode(AppMode::Guest).build().unwrap();

    let mut child_window_opts = WindowOptions::new();
    unsafe {
        child_window_opts.raw_parent(parent_window.as_raw().unwrap());
    }
    let child_window = child_window_opts
        .position(Point::new(128.0, 128.0))
        .size(Size::new(256.0, 256.0))
        .open(child_app.handle(), {
            let mut framebuffer = Vec::new();
            move |cx, event| {
                match event {
                    Event::Frame => {
                        let scale = cx.window().scale();
                        let size = cx.window().size();
                        let width = (scale * size.width) as usize;
                        let height = (scale * size.height) as usize;
                        framebuffer.resize(width * height, 0xFFFF00FF);
                        cx.window().present(Bitmap::new(&framebuffer, width, height));
                    }
                    _ => {}
                }

                Response::Ignore
            }
        })
        .unwrap();

    parent_window.show();
    child_window.show();

    parent_app.run().unwrap();

    child_window.close();
    parent_window.close();
}
