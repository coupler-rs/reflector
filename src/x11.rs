use crate::{
    App, AppContext, CloseError, Cursor, Event, Rect, Response, Result, Window, WindowOptions,
};

use std::fmt;
use std::marker::PhantomData;

use raw_window_handle::RawWindowHandle;

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

    pub fn run(&self) -> Result<()> {
        Ok(())
    }

    pub fn poll(&self) -> Result<()> {
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

    pub fn request_display(&self) {}

    pub fn request_display_rect(&self, rect: Rect) {}

    pub fn update_contents(&self, framebuffer: &[u32], width: usize, height: usize) {}

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        unimplemented!()
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        unimplemented!()
    }
}
