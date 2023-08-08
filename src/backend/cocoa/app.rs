use std::marker::PhantomData;
use std::result;
use std::time::Duration;

use super::TimerHandleInner;
use crate::{App, AppContext, IntoInnerError, Result};

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
            _marker: PhantomData,
        });

        Ok(AppInner { state: build(&cx)? })
    }

    pub fn run(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>> {
        unimplemented!()
    }
}

pub struct AppContextInner<'a, T> {
    _marker: PhantomData<&'a T>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerHandleInner
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        TimerHandleInner {}
    }

    pub fn exit(&self) {}
}
