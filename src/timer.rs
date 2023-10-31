use std::fmt;
use std::marker::PhantomData;

use crate::{backend, AppHandle};

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
