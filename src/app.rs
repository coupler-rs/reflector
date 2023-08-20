use std::marker::PhantomData;
use std::time::Duration;
use std::{fmt, result};

use crate::{backend, IntoInnerError, Result};

pub struct TimerHandle {
    inner: backend::TimerHandleInner,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl TimerHandle {
    pub fn cancel(self) {
        self.inner.cancel();
    }
}

impl fmt::Debug for TimerHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("TimerHandle").finish_non_exhaustive()
    }
}

pub struct App<T> {
    inner: backend::AppInner<T>,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl<T: 'static> App<T> {
    pub(crate) fn from_inner(inner: backend::AppInner<T>) -> App<T> {
        App {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn new<F>(build: F) -> Result<App<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        Ok(App::from_inner(backend::AppInner::new(build)?))
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }

    pub fn poll(&mut self) -> Result<()> {
        self.inner.poll()
    }

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>> {
        self.inner.into_inner()
    }
}

impl<T> fmt::Debug for App<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("App").finish_non_exhaustive()
    }
}

#[cfg(target_os = "linux")]
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(target_os = "linux")]
impl<T> AsRawFd for App<T> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

pub struct AppContext<'a, T> {
    pub(crate) inner: backend::AppContextInner<'a, T>,
    // ensure !Send and !Sync on all platforms
    _marker: PhantomData<*mut ()>,
}

impl<'a, T: 'static> AppContext<'a, T> {
    pub(crate) fn from_inner(inner: backend::AppContextInner<T>) -> AppContext<T> {
        AppContext {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerHandle
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        TimerHandle {
            inner: self.inner.set_timer(duration, handler),
            _marker: PhantomData,
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
