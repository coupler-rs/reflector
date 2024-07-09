use reflector_platform as platform;

pub use platform::{Point, Size};

mod error;

pub use error::{Error, Result};

pub struct App {
    inner: platform::App,
}

impl App {
    pub fn new() -> Result<App> {
        Ok(App {
            inner: platform::App::new()?,
        })
    }

    pub fn run(&self) -> Result<()> {
        Ok(self.inner.run()?)
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

    pub fn open(&self, app: &App) -> Result<Window> {
        let window = self.inner.open(app.inner.handle(), |_, _| platform::Response::Ignore)?;

        window.show();

        Ok(Window { _inner: window })
    }
}

pub struct Window {
    _inner: platform::Window,
}
