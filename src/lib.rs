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

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Os(platform::OsError),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Os(err) => write!(fmt, "os error: {}", err),
        }
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

    pub fn run(&mut self) {
        self.inner.run();
    }

    pub fn poll(&mut self) {
        self.inner.poll();
    }
}

impl<T> fmt::Debug for App<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("App").finish_non_exhaustive()
    }
}

pub struct AppContext<T> {
    inner: platform::AppContextInner<T>,
    // ensure !Send and !Sync on all platforms
    phantom: PhantomData<*mut ()>,
}

impl<T> AppContext<T> {
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

impl<T> fmt::Debug for AppContext<T> {
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
    RequestClose,
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
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        H: 'static,
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

impl fmt::Debug for Window {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Window").finish_non_exhaustive()
    }
}
