pub use platform::AppMode;

use crate::Result;

#[derive(Default)]
pub struct AppOptions {
    inner: platform::AppOptions,
}

impl AppOptions {
    pub fn new() -> AppOptions {
        Self::default()
    }

    pub fn mode(&mut self, mode: AppMode) -> &mut Self {
        self.inner.mode(mode);
        self
    }

    pub fn build(&self) -> Result<App> {
        Ok(App {
            inner: self.inner.build()?,
        })
    }
}

pub struct App {
    pub(crate) inner: platform::App,
}

impl App {
    pub fn new() -> Result<App> {
        AppOptions::default().build()
    }

    pub fn run(&self) -> Result<()> {
        Ok(self.inner.run()?)
    }
}
