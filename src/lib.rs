use reflector_platform as platform;

mod error;

pub use error::{Error, Result};

pub struct App {
    platform_app: platform::App,
}

impl App {
    pub fn new() -> Result<App> {
        Ok(App {
            platform_app: platform::App::new()?,
        })
    }

    pub fn run(&self) -> Result<()> {
        Ok(self.platform_app.run()?)
    }
}
