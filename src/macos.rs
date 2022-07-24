use crate::{AppContext, Event, Response, Result, WindowOptions};

use std::fmt;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct OsError {}

impl fmt::Display for OsError {
    fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

pub struct AppInner<T> {
    state: T,
}

impl<T> AppInner<T> {
    pub fn new<F>(build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        let cx = AppContext::from_inner(AppContextInner {
            phantom: PhantomData,
        });

        Ok(AppInner { state: build(&cx)? })
    }

    pub fn run(&self) {}

    pub fn poll(&self) {}
}

pub struct AppContextInner<'a, T> {
    phantom: PhantomData<&'a T>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub fn exit(&self) {}
}

pub struct WindowInner {}

impl WindowInner {
    pub fn open<T, H>(
        _options: &WindowOptions,
        _cx: &AppContext<T>,
        _handler: H,
    ) -> Result<WindowInner>
    where
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        H: 'static,
    {
        Ok(WindowInner {})
    }
}
