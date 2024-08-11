use graphics::{Affine, Color, Renderer};
use platform::{Bitmap, WindowContext};

use crate::{App, Build, Context, Elem, Event, Point, ProposedSize, Result, Size};

struct Handler<E> {
    renderer: Renderer,
    framebuffer: Vec<u32>,
    root: E,
}

impl<E: Elem> Handler<E> {
    fn new(root: E) -> Handler<E> {
        Handler {
            renderer: Renderer::new(),
            framebuffer: Vec::new(),
            root,
        }
    }

    fn handle(&mut self, cx: &WindowContext, event: platform::Event) -> platform::Response {
        match event {
            platform::Event::Frame => {
                let scale = cx.window().scale() as f32;
                let size = cx.window().size();
                let size = Size::new(size.width as f32, size.height as f32);

                self.root.update(&mut Context {});
                let root_size = self.root.measure(&mut Context {}, ProposedSize::from(size));
                self.root.place(&mut Context {}, root_size);

                let width = (scale * size.width) as usize;
                let height = (scale * size.height) as usize;
                self.framebuffer.resize(width * height, 0xFF000000);

                let mut canvas = self.renderer.canvas(&mut self.framebuffer, width, height);
                canvas.clear(Color::rgba(255, 255, 255, 255));

                canvas.with_transform(Affine::scale(scale as f32), |canvas| {
                    self.root.render(&mut Context {}, canvas);
                });

                cx.window().present(Bitmap::new(&self.framebuffer, width, height));
            }
            platform::Event::Close => {
                cx.window().close();
                cx.app().exit();
            }
            platform::Event::MouseMove(pos) => {
                let pos = Point::new(pos.x as f32, pos.y as f32);
                self.root.handle(&mut Context {}, &Event::MouseMove(pos));
            }
            platform::Event::MouseDown(button) => {
                self.root.handle(&mut Context {}, &Event::MouseDown(button));
            }
            platform::Event::MouseUp(button) => {
                self.root.handle(&mut Context {}, &Event::MouseUp(button));
            }
            platform::Event::Scroll(delta) => {
                let delta = Point::new(delta.x as f32, delta.y as f32);
                self.root.handle(&mut Context {}, &Event::Scroll(delta));
            }
            _ => {}
        }

        platform::Response::Ignore
    }
}

#[derive(Default)]
pub struct WindowOptions {
    inner: platform::WindowOptions,
}

impl WindowOptions {
    pub fn new() -> WindowOptions {
        Self::default()
    }

    pub fn title<S: AsRef<str>>(&mut self, title: S) -> &mut Self {
        self.inner.title(title);
        self
    }

    pub fn position(&mut self, position: Point) -> &mut Self {
        self.inner.position(platform::Point::new(position.x as f64, position.y as f64));
        self
    }

    pub fn size(&mut self, size: Size) -> &mut Self {
        self.inner.size(platform::Size::new(size.width as f64, size.height as f64));
        self
    }

    pub fn open<B>(&self, app: &App, root: B) -> Result<Window>
    where
        B: Build,
        B::Elem: 'static,
    {
        let mut handler = Handler::new(root.build(&mut Context {}));

        let window = self.inner.open(app.inner.handle(), move |cx, event| {
            handler.handle(cx, event)
        })?;

        window.show();

        Ok(Window { _inner: window })
    }
}

pub struct Window {
    _inner: platform::Window,
}
