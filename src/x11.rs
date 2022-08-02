use crate::{
    App, AppContext, CloseError, Cursor, Error, Event, Point, Rect, Response, Result, Window,
    WindowOptions,
};

use std::marker::PhantomData;
use std::os::raw::c_int;
use std::rc::Rc;
use std::{fmt, ptr, result};

use raw_window_handle::RawWindowHandle;
use xcb_sys as xcb;

#[derive(Debug)]
pub struct OsError {
    code: c_int,
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.code)
    }
}

struct AppState {
    connection: *mut xcb::xcb_connection_t,
    screen: *mut xcb::xcb_screen_t,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            xcb::xcb_disconnect(self.connection);
        }
    }
}

pub struct AppInner<T> {
    state: Rc<AppState>,
    data: Box<T>,
}

impl<T> AppInner<T> {
    pub fn new<F>(build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        let state = unsafe {
            let mut default_screen_index = 0;
            let connection = xcb::xcb_connect(ptr::null(), &mut default_screen_index);

            let error = xcb::xcb_connection_has_error(connection);
            if error != 0 {
                xcb::xcb_disconnect(connection);
                return Err(Error::Os(OsError { code: error }));
            }

            let setup = xcb::xcb_get_setup(connection);
            let mut roots_iter = xcb::xcb_setup_roots_iterator(setup);
            for _ in 0..default_screen_index {
                xcb::xcb_screen_next(&mut roots_iter);
            }
            let screen = roots_iter.data;

            AppState { connection, screen }
        };

        let cx = AppContext::from_inner(AppContextInner {
            phantom: PhantomData,
        });

        Ok(AppInner {
            state: Rc::new(state),
            data: Box::new(build(&cx)?),
        })
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

    pub fn set_mouse_position(&self, position: Point) {}

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        unimplemented!()
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        unimplemented!()
    }
}
