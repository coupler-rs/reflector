use graphics::{Affine, Color, Renderer};
use platform::{Bitmap, WindowContext};

use crate::{App, Build, Constraints, Context, Elem, Point, Result, Size};

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
                let scale = cx.window().scale();
                let size = cx.window().size();

                self.root.update(&mut Context {});

                let constraints = Constraints {
                    min: size,
                    max: size,
                };
                self.root.layout(&mut Context {}, constraints);

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
        self.inner.position(position);
        self
    }

    pub fn size(&mut self, size: Size) -> &mut Self {
        self.inner.size(size);
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
