use graphics::{Affine, Color, Renderer};
use platform::{Bitmap, RawWindow, WindowContext};

use crate::{App, BuildElem, Elem, ElemContext, ElemEvent, Point, ProposedSize, Result, Size};

struct Handler<E> {
    renderer: Renderer,
    framebuffer: Vec<u32>,
    hover: bool,
    root: E,
}

impl<E: Elem> Handler<E> {
    fn new(root: E) -> Handler<E> {
        Handler {
            renderer: Renderer::new(),
            framebuffer: Vec::new(),
            hover: false,
            root,
        }
    }

    fn handle(&mut self, cx: &WindowContext, event: platform::Event) -> platform::Response {
        match event {
            platform::Event::Frame => {
                let scale = cx.window().scale() as f32;
                let size = cx.window().size();
                let size = Size::new(size.width as f32, size.height as f32);

                self.root.update(&mut ElemContext {});
                let root_size = self.root.measure(&mut ElemContext {}, ProposedSize::from(size));
                self.root.place(&mut ElemContext {}, root_size);

                let width = (scale * size.width) as usize;
                let height = (scale * size.height) as usize;
                self.framebuffer.resize(width * height, 0xFF000000);

                let mut canvas = self.renderer.canvas(&mut self.framebuffer, width, height);
                canvas.clear(Color::rgba(255, 255, 255, 255));

                canvas.with_transform(Affine::scale(scale), |canvas| {
                    self.root.render(&mut ElemContext {}, canvas);
                });

                cx.window().present(Bitmap::new(&self.framebuffer, width, height));
            }
            platform::Event::Close => {
                cx.window().close();
                cx.event_loop().exit();
            }
            platform::Event::MouseExit => {
                if self.hover {
                    self.hover = false;
                    self.root.handle(&mut ElemContext {}, &ElemEvent::MouseExit);
                }
            }
            platform::Event::MouseMove(pos) => {
                let pos = Point::new(pos.x as f32, pos.y as f32);

                #[allow(clippy::collapsible_else_if)]
                if self.root.hit_test(&mut ElemContext {}, pos) {
                    if !self.hover {
                        self.hover = true;
                        self.root.handle(&mut ElemContext {}, &ElemEvent::MouseEnter);
                    }

                    self.root.handle(&mut ElemContext {}, &ElemEvent::MouseMove(pos));
                } else {
                    if self.hover {
                        self.hover = false;
                        self.root.handle(&mut ElemContext {}, &ElemEvent::MouseExit);
                    }
                }
            }
            platform::Event::MouseDown(button) => {
                if self.hover {
                    self.root.handle(&mut ElemContext {}, &ElemEvent::MouseDown(button));
                }
            }
            platform::Event::MouseUp(button) => {
                if self.hover {
                    self.root.handle(&mut ElemContext {}, &ElemEvent::MouseUp(button));
                }
            }
            platform::Event::Scroll(delta) => {
                if self.hover {
                    let delta = Point::new(delta.x as f32, delta.y as f32);
                    self.root.handle(&mut ElemContext {}, &ElemEvent::Scroll(delta));
                }
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

    pub unsafe fn raw_parent(&mut self, parent: RawWindow) -> &mut Self {
        self.inner.raw_parent(parent);
        self
    }

    pub fn open<B>(&self, app: &App, root: B) -> Result<Window>
    where
        B: BuildElem,
        B::Elem: 'static,
    {
        let mut handler = Handler::new(root.build(&mut ElemContext {}));

        let window = self.inner.open(app.event_loop.handle(), move |cx, event| {
            handler.handle(cx, event)
        })?;

        window.show();

        Ok(Window { _inner: window })
    }
}

pub struct Window {
    _inner: platform::Window,
}
