use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use crate::{backend, Result};

pub struct TimerContext<'a> {
    app: &'a AppHandle,
    timer: &'a Timer,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl<'a> TimerContext<'a> {
    pub(crate) fn new(app: &'a AppHandle, timer: &'a Timer) -> TimerContext<'a> {
        TimerContext {
            app,
            timer,
            _marker: PhantomData,
        }
    }

    pub fn app(&self) -> &AppHandle {
        self.app
    }

    pub fn timer(&self) -> &Timer {
        self.timer
    }
}

#[derive(Clone)]
pub struct Timer {
    pub(crate) inner: backend::TimerInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl Timer {
    pub(crate) fn from_inner(inner: backend::TimerInner) -> Timer {
        Timer {
            inner,
            _marker: PhantomData,
        }
    }

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
    handle: AppHandle,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl App {
    pub(crate) fn from_inner(inner: backend::AppInner) -> App {
        App {
            handle: AppHandle::from_inner(inner),
            _marker: PhantomData,
        }
    }

    pub fn handle(&self) -> &AppHandle {
        &self.handle
    }

    pub fn run(&self) -> Result<()> {
        self.handle.inner.run()
    }

    pub fn poll(&self) -> Result<()> {
        self.handle.inner.poll()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.handle.inner.shutdown();
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
        self.handle.inner.as_raw_fd()
    }
}

pub struct AppHandle {
    pub(crate) inner: backend::AppInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl AppHandle {
    pub(crate) fn from_inner(inner: backend::AppInner) -> AppHandle {
        AppHandle {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> Timer
    where
        H: FnMut(&TimerContext) + 'static,
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

impl fmt::Debug for AppHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("AppHandle").finish_non_exhaustive()
    }
}
