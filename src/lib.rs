#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "linux")]
use x11 as platform;

use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::{error, fmt, result};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Os(platform::OsError),
    InsideEventHandler,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Os(err) => write!(fmt, "os error: {}", err),
            Error::InsideEventHandler => {
                write!(fmt, "operation not supported inside an event handler")
            }
        }
    }
}

#[derive(Debug)]
pub struct CloseError<T> {
    error: Error,
    inner: T,
}

impl<T> CloseError<T> {
    fn new(error: Error, inner: T) -> CloseError<T> {
        CloseError { error, inner }
    }

    pub fn error(&self) -> &Error {
        &self.error
    }

    pub fn into_error(self) -> Error {
        self.error
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn into_parts(self) -> (Error, T) {
        (self.error, self.inner)
    }
}

impl<T: Send + fmt::Debug> error::Error for CloseError<T> {}

impl<T> fmt::Display for CloseError<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(fmt)
    }
}

pub struct App<T> {
    inner: platform::AppInner<T>,
    // ensure !Send and !Sync on all platforms
    phantom: PhantomData<*mut ()>,
}

impl<T> App<T> {
    pub fn new<F>(build: F) -> Result<App<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        Ok(App {
            inner: platform::AppInner::new(build)?,
            phantom: PhantomData,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }

    pub fn poll(&mut self) -> Result<()> {
        self.inner.poll()
    }

    pub fn into_inner(self) -> result::Result<T, CloseError<App<T>>> {
        self.inner.into_inner()
    }
}

impl<T> fmt::Debug for App<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("App").finish_non_exhaustive()
    }
}

pub struct AppContext<'a, T> {
    inner: platform::AppContextInner<'a, T>,
    // ensure !Send and !Sync on all platforms
    phantom: PhantomData<*mut ()>,
}

impl<'a, T> AppContext<'a, T> {
    fn from_inner(inner: platform::AppContextInner<T>) -> AppContext<T> {
        AppContext {
            inner,
            phantom: PhantomData,
        }
    }

    pub fn exit(&self) {
        self.inner.exit();
    }
}

impl<'a, T> fmt::Debug for AppContext<'a, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("AppContext").finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
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
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
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
pub enum Event {
    Frame,
    Display,
    MouseMove(Point),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    Scroll(Point),
    RequestClose,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Response {
    Capture,
    Ignore,
}

#[derive(Clone, Debug)]
pub struct WindowOptions {
    title: OsString,
    rect: Rect,
}

impl Default for WindowOptions {
    fn default() -> Self {
        WindowOptions {
            title: OsString::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
        }
    }
}

impl WindowOptions {
    pub fn new() -> WindowOptions {
        Self::default()
    }

    pub fn title<S: AsRef<OsStr>>(&mut self, title: S) -> &mut Self {
        self.title = title.as_ref().to_os_string();
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

    pub fn open<T, H>(&self, cx: &AppContext<T>, handler: H) -> Result<Window>
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        T: 'static,
    {
        Ok(Window {
            inner: platform::WindowInner::open(self, cx, handler)?,
            phantom: PhantomData,
        })
    }
}

pub struct Window {
    inner: platform::WindowInner,
    // ensure !Send and !Sync on all platforms
    phantom: PhantomData<*mut ()>,
}

impl Window {
    pub fn show(&self) {
        self.inner.show();
    }

    pub fn hide(&self) {
        self.inner.hide();
    }

    pub fn request_display(&self) {
        self.inner.request_display();
    }

    pub fn request_display_rect(&self, rect: Rect) {
        self.inner.request_display_rect(rect);
    }

    pub fn update_contents(&self, buffer: &[u32], width: usize, height: usize) {
        self.inner.update_contents(buffer, width, height);
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        self.inner.set_cursor(cursor);
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        self.inner.close()
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
