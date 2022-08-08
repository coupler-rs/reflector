use crate::{
    App, AppContext, Bitmap, CloseError, Cursor, Event, Point, Rect, Response, Result, Window,
    WindowOptions,
};

use std::marker::PhantomData;
use std::time::Duration;
use std::{fmt, result};

use raw_window_handle::RawWindowHandle;

#[derive(Debug)]
pub struct OsError {}

impl fmt::Display for OsError {
    fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

pub struct TimerHandleInner {}

impl TimerHandleInner {
    pub fn cancel(self) {}
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

    pub fn run(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn into_inner(self) -> result::Result<T, CloseError<App<T>>> {
        unimplemented!()
    }
}

pub struct AppContextInner<'a, T> {
    phantom: PhantomData<&'a T>,
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

    pub fn show(&self) {}

    pub fn hide(&self) {}

    pub fn present(&self, bitmap: Bitmap) {}

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {}

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, position: Point) {}

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        unimplemented!()
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        unimplemented!()
    }
}
