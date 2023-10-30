use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use crate::{backend, Result};

#[derive(Clone)]
pub struct Timer {
    inner: backend::TimerInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl Timer {
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Timer").finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AppMode {
    Owner,
    Guest,
}

#[derive(Clone, Debug)]
pub struct AppOptions {
    pub(crate) mode: AppMode,
}

impl Default for AppOptions {
    fn default() -> Self {
        AppOptions {
            mode: AppMode::Owner,
        }
    }
}

impl AppOptions {
    pub fn new() -> AppOptions {
        Self::default()
    }

    pub fn mode(&mut self, mode: AppMode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn build(&self) -> Result<App> {
        Ok(App::from_inner(backend::AppInner::new(self)?))
    }
}

pub struct App {
    inner: backend::AppInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl App {
    pub(crate) fn from_inner(inner: backend::AppInner) -> App {
        App {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn context(&self) -> AppContext {
        AppContext::from_inner(self.inner.context())
    }

    pub fn run(&self) -> Result<()> {
        self.inner.run()
    }

    pub fn poll(&self) -> Result<()> {
        self.inner.poll()
    }
}

impl fmt::Debug for App {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("App").finish_non_exhaustive()
    }
}

#[cfg(target_os = "linux")]
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(target_os = "linux")]
impl AsRawFd for App {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

pub struct AppContext<'a> {
    pub(crate) inner: backend::AppContextInner<'a>,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl<'a> AppContext<'a> {
    pub(crate) fn from_inner(inner: backend::AppContextInner) -> AppContext {
        AppContext {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> Timer
    where
        H: FnMut(&AppContext) + 'static,
    {
        Timer {
            inner: self.inner.set_timer(duration, handler),
            _marker: PhantomData,
        }
    }

    pub fn exit(&self) {
        self.inner.exit();
    }
}

impl<'a> fmt::Debug for AppContext<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("AppContext").finish_non_exhaustive()
    }
}
