use crate::{
    App, AppContext, CloseError, Cursor, Error, Event, Point, Rect, Response, Result, Window,
    WindowOptions,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::rc::Rc;
use std::{fmt, mem, ptr, result};

use raw_window_handle::{unix::XcbHandle, HasRawWindowHandle, RawWindowHandle};
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

unsafe fn intern_atom(
    connection: *mut xcb::xcb_connection_t,
    name: &[u8],
) -> xcb::xcb_intern_atom_cookie_t {
    xcb::xcb_intern_atom(
        connection,
        0,
        name.len() as u16,
        name.as_ptr() as *const c_char,
    )
}

unsafe fn intern_atom_reply(
    connection: *mut xcb::xcb_connection_t,
    cookie: xcb::xcb_intern_atom_cookie_t,
) -> Result<xcb::xcb_atom_t> {
    let mut error = ptr::null_mut();
    let reply = xcb::xcb_intern_atom_reply(connection, cookie, &mut error);

    if !error.is_null() {
        let error_code = (*error).error_code;
        libc::free(error as *mut c_void);
        return Err(Error::Os(OsError {
            code: error_code as c_int,
        }));
    }

    if reply.is_null() {
        let error = xcb::xcb_connection_has_error(connection);
        return Err(Error::Os(OsError { code: error }));
    }

    let atom = (*reply).atom;
    libc::free(reply as *mut c_void);

    Ok(atom)
}

struct Atoms {
    wm_protocols: xcb::xcb_atom_t,
    wm_delete_window: xcb::xcb_atom_t,
}

impl Atoms {
    unsafe fn new(connection: *mut xcb::xcb_connection_t) -> Result<Atoms> {
        let wm_protocols_cookie = intern_atom(connection, b"WM_PROTOCOLS");
        let wm_delete_window_cookie = intern_atom(connection, b"WM_DELETE_WINDOW");

        let wm_protocols = intern_atom_reply(connection, wm_protocols_cookie)?;
        let wm_delete_window = intern_atom_reply(connection, wm_delete_window_cookie)?;

        Ok(Atoms {
            wm_protocols,
            wm_delete_window,
        })
    }
}

trait RemoveHandler {
    fn remove_handler(&self, window_id: xcb::xcb_window_t);
}

type Handler<T> = Rc<RefCell<dyn FnMut(&mut T, &AppContext<T>, Event) -> Response>>;

struct Handlers<T>(RefCell<HashMap<xcb::xcb_window_t, Handler<T>>>);

impl<T> RemoveHandler for Handlers<T> {
    fn remove_handler(&self, window_id: xcb::xcb_window_t) {
        self.0.borrow_mut().remove(&window_id);
    }
}

struct AppState<H: ?Sized> {
    connection: *mut xcb::xcb_connection_t,
    screen: *mut xcb::xcb_screen_t,
    atoms: Atoms,
    handlers: H,
}

impl<H: ?Sized> Drop for AppState<H> {
    fn drop(&mut self) {
        unsafe {
            xcb::xcb_disconnect(self.connection);
        }
    }
}

pub struct AppInner<T> {
    state: Rc<AppState<Handlers<T>>>,
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

            let atoms = match Atoms::new(connection) {
                Ok(atoms) => atoms,
                Err(err) => {
                    xcb::xcb_disconnect(connection);
                    return Err(err);
                }
            };

            Rc::new(AppState {
                connection,
                screen,
                atoms,
                handlers: Handlers(RefCell::new(HashMap::new())),
            })
        };

        let cx = AppContext::from_inner(AppContextInner { state: &state });
        let data = build(&cx)?;

        let mut inner = AppInner {
            state,
            data: Box::new(data),
        };

        Ok(inner)
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
    state: &'a Rc<AppState<Handlers<T>>>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub fn exit(&self) {}
}

pub struct WindowInner {
    window_id: xcb::xcb_window_t,
    app_state: Rc<AppState<dyn RemoveHandler>>,
}

impl WindowInner {
    pub fn open<T, H>(
        options: &WindowOptions,
        cx: &AppContext<T>,
        handler: H,
    ) -> Result<WindowInner>
    where
        T: 'static,
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        H: 'static,
    {
        let window_id = unsafe {
            let window_id = xcb::xcb_generate_id(cx.inner.state.connection);

            let parent_id = (*cx.inner.state.screen).root;

            let value_mask = xcb::XCB_CW_EVENT_MASK;
            let value_list = &[0];

            let cookie = xcb::xcb_create_window_checked(
                cx.inner.state.connection,
                xcb::XCB_COPY_FROM_PARENT as u8,
                window_id,
                parent_id,
                options.rect.x as i16,
                options.rect.y as i16,
                options.rect.width as u16,
                options.rect.height as u16,
                0,
                xcb::XCB_WINDOW_CLASS_INPUT_OUTPUT as u16,
                xcb::XCB_COPY_FROM_PARENT,
                value_mask,
                value_list.as_ptr() as *const c_void,
            );

            let error = xcb::xcb_request_check(cx.inner.state.connection, cookie);
            if !error.is_null() {
                let error_code = (*error).error_code;
                libc::free(error as *mut c_void);
                return Err(Error::Os(OsError {
                    code: error_code as c_int,
                }));
            }

            let atoms = &[cx.inner.state.atoms.wm_delete_window];
            xcb::xcb_icccm_set_wm_protocols(
                cx.inner.state.connection,
                window_id,
                cx.inner.state.atoms.wm_protocols,
                atoms.len() as u32,
                atoms.as_ptr() as *mut xcb::xcb_atom_t,
            );

            xcb::xcb_flush(cx.inner.state.connection);

            let handler = Rc::new(RefCell::new(handler));
            let handlers = &cx.inner.state.handlers;
            handlers.0.borrow_mut().insert(window_id, handler);

            window_id
        };

        Ok(WindowInner {
            window_id,
            app_state: cx.inner.state.clone(),
        })
    }

    pub fn show(&self) {
        unsafe {
            let cookie = xcb::xcb_map_window_checked(self.app_state.connection, self.window_id);

            let error = xcb::xcb_request_check(self.app_state.connection, cookie);
            if !error.is_null() {
                libc::free(error as *mut c_void);
            }
        }
    }

    pub fn hide(&self) {}

    pub fn request_display(&self) {}

    pub fn request_display_rect(&self, rect: Rect) {}

    pub fn update_contents(&self, framebuffer: &[u32], width: usize, height: usize) {}

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, position: Point) {}

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Xcb(XcbHandle {
            window: self.window_id,
            connection: self.app_state.connection as *mut c_void,
            ..XcbHandle::empty()
        })
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        if let Err(error) = self.destroy() {
            return Err(CloseError::new(error, Window::from_inner(self)));
        }

        mem::forget(self);

        Ok(())
    }

    fn destroy(&self) -> Result<()> {
        unsafe {
            let cookie = xcb::xcb_destroy_window_checked(self.app_state.connection, self.window_id);
            let error = xcb::xcb_request_check(self.app_state.connection, cookie);

            if !error.is_null() {
                let error_code = (*error).error_code;
                libc::free(error as *mut c_void);
                return Err(Error::Os(OsError {
                    code: error_code as c_int,
                }));
            }
        }

        self.app_state.handlers.remove_handler(self.window_id);

        Ok(())
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}
