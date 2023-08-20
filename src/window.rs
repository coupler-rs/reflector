use std::fmt;
use std::marker::PhantomData;

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{backend, AppContext, Result};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[inline]
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    #[inline]
    pub fn new(width: f64, height: f64) -> Size {
        Size { width, height }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[inline]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct Bitmap<'a> {
    data: &'a [u32],
    width: usize,
    height: usize,
}

impl<'a> Bitmap<'a> {
    #[inline]
    pub fn new(data: &'a [u32], width: usize, height: usize) -> Bitmap<'a> {
        assert!(width * height == data.len(), "invalid bitmap dimensions");

        Bitmap {
            data,
            width,
            height,
        }
    }

    #[inline]
    pub fn data(&self) -> &'a [u32] {
        self.data
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Cursor {
    Arrow,
    Crosshair,
    Hand,
    IBeam,
    No,
    SizeNs,
    SizeWe,
    SizeNesw,
    SizeNwse,
    Wait,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event<'a> {
    Expose(&'a [Rect]),
    Close,
    MouseMove(Point),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    Scroll(Point),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Response {
    Capture,
    Ignore,
}

#[derive(Clone, Debug)]
pub(crate) enum Parent<'a> {
    Window(&'a Window),
    Raw(RawWindowHandle),
}

#[derive(Clone, Debug)]
pub struct WindowOptions<'a> {
    pub(crate) title: String,
    pub(crate) rect: Rect,
    pub(crate) parent: Option<Parent<'a>>,
}

impl<'a> Default for WindowOptions<'a> {
    fn default() -> Self {
        WindowOptions {
            title: String::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            parent: None,
        }
    }
}

impl<'a> WindowOptions<'a> {
    pub fn new() -> WindowOptions<'a> {
        Self::default()
    }

    pub fn title<S: AsRef<str>>(&mut self, title: S) -> &mut Self {
        self.title = title.as_ref().to_string();
        self
    }

    pub fn rect(&mut self, rect: Rect) -> &mut Self {
        self.rect = rect;
        self
    }

    pub fn position(&mut self, point: Point) -> &mut Self {
        self.rect.x = point.x;
        self.rect.y = point.y;
        self
    }

    pub fn size(&mut self, size: Size) -> &mut Self {
        self.rect.width = size.width;
        self.rect.height = size.height;
        self
    }

    pub fn parent(&mut self, parent: &'a Window) -> &mut Self {
        self.parent = Some(Parent::Window(parent));
        self
    }

    pub unsafe fn raw_parent(&mut self, parent: RawWindowHandle) -> &mut Self {
        self.parent = Some(Parent::Raw(parent));
        self
    }

    pub fn open<T, H>(&self, cx: &AppContext<T>, handler: H) -> Result<Window>
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        T: 'static,
    {
        Ok(Window::from_inner(backend::WindowInner::open(
            self, cx, handler,
        )?))
    }
}

pub struct Window {
    inner: backend::WindowInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl Window {
    fn from_inner(inner: backend::WindowInner) -> Window {
        Window {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn show(&self) {
        self.inner.show();
    }

    pub fn hide(&self) {
        self.inner.hide();
    }

    pub fn present(&self, bitmap: Bitmap) {
        self.inner.present(bitmap);
    }

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {
        self.inner.present_partial(bitmap, rects);
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        self.inner.set_cursor(cursor);
    }

    pub fn set_mouse_position(&self, position: Point) {
        self.inner.set_mouse_position(position);
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Window").finish_non_exhaustive()
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner.raw_window_handle()
    }
}
