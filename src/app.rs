use crate::Result;

pub struct App {
    pub(crate) inner: platform::App,
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
