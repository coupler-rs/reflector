pub use platform::{EventLoop, EventLoopOptions, Mode};

use crate::Result;

#[derive(Default)]
pub struct AppOptions {
    event_loop_options: EventLoopOptions,
}

impl AppOptions {
    pub fn new() -> AppOptions {
        Self::default()
    }

    pub fn mode(&mut self, mode: Mode) -> &mut Self {
        self.event_loop_options.mode(mode);
        self
    }

    pub fn build(&self) -> Result<App> {
        Ok(App {
            event_loop: self.event_loop_options.build()?,
        })
    }
}

pub struct App {
    pub(crate) event_loop: EventLoop,
}

impl App {
    pub fn new() -> Result<App> {
        AppOptions::default().build()
    }

    pub fn run(&self) -> Result<()> {
        Ok(self.event_loop.run()?)
    }
}
