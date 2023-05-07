use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Rc;
use std::time::Duration;
use std::{ptr, result};

use xcb_sys as xcb;

use super::window::WindowState;
use super::{OsError, TimerHandleInner};
use crate::{
    App, AppContext, Cursor, Error, Event, IntoInnerError, MouseButton, Point, Rect, Result,
};

fn mouse_button_from_code(code: xcb::xcb_button_t) -> Option<MouseButton> {
    match code {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        8 => Some(MouseButton::Back),
        9 => Some(MouseButton::Forward),
        _ => None,
    }
}

fn scroll_delta_from_code(code: xcb::xcb_button_t) -> Option<Point> {
    match code {
        4 => Some(Point::new(0.0, 1.0)),
        5 => Some(Point::new(0.0, -1.0)),
        6 => Some(Point::new(-1.0, 0.0)),
        7 => Some(Point::new(1.0, 0.0)),
        _ => None,
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
        return Err(Error::Os(OsError::Xcb(error_code as c_int)));
    }

    if reply.is_null() {
        let error = xcb::xcb_connection_has_error(connection);
        return Err(Error::Os(OsError::Xcb(error)));
    }

    let atom = (*reply).atom;
    libc::free(reply as *mut c_void);

    Ok(atom)
}

pub struct Atoms {
    pub wm_protocols: xcb::xcb_atom_t,
    pub wm_delete_window: xcb::xcb_atom_t,
    pub _net_wm_name: xcb::xcb_atom_t,
    pub utf8_string: xcb::xcb_atom_t,
}

impl Atoms {
    unsafe fn new(connection: *mut xcb::xcb_connection_t) -> Result<Atoms> {
        let wm_protocols_cookie = intern_atom(connection, b"WM_PROTOCOLS");
        let wm_delete_window_cookie = intern_atom(connection, b"WM_DELETE_WINDOW");
        let _net_wm_name_cookie = intern_atom(connection, b"_NET_WM_NAME");
        let utf8_string_cookie = intern_atom(connection, b"UTF8_STRING");

        let wm_protocols = intern_atom_reply(connection, wm_protocols_cookie)?;
        let wm_delete_window = intern_atom_reply(connection, wm_delete_window_cookie)?;
        let _net_wm_name = intern_atom_reply(connection, _net_wm_name_cookie)?;
        let utf8_string = intern_atom_reply(connection, utf8_string_cookie)?;

        Ok(Atoms {
            wm_protocols,
            wm_delete_window,
            _net_wm_name,
            utf8_string,
        })
    }
}

pub struct AppState {
    pub connection: *mut xcb::xcb_connection_t,
    pub screen: *mut xcb::xcb_screen_t,
    pub atoms: Atoms,
    pub shm_supported: bool,
    pub cursor_context: *mut xcb::xcb_cursor_context_t,
    pub cursor_cache: RefCell<HashMap<Cursor, xcb::xcb_cursor_t>>,
    pub running: Cell<bool>,
    pub windows: RefCell<HashMap<xcb::xcb_window_t, Rc<WindowState>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            for (_window_id, window) in self.windows.take().drain() {
                window.destroy(&self);
            }

            if let Some(cursor_id) = self.cursor_cache.borrow().get(&Cursor::None) {
                xcb::xcb_free_cursor(self.connection, *cursor_id);
            }
            xcb::xcb_cursor_context_free(self.cursor_context);

            xcb::xcb_flush(self.connection);
            xcb::xcb_disconnect(self.connection);
        }
    }
}

pub struct AppInner<T> {
    state: Rc<AppState>,
    data: Box<T>,
}

impl<T: 'static> AppInner<T> {
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
                return Err(Error::Os(OsError::Xcb(error)));
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

            let shm_cookie = xcb::xcb_shm_query_version(connection);
            let shm_version =
                xcb::xcb_shm_query_version_reply(connection, shm_cookie, ptr::null_mut());
            let shm_supported = !shm_version.is_null();
            if shm_supported {
                libc::free(shm_version as *mut c_void);
            }

            let mut cursor_context = ptr::null_mut();
            let error = xcb::xcb_cursor_context_new(connection, screen, &mut cursor_context);
            if error < 0 {
                xcb::xcb_disconnect(connection);
                return Err(Error::Os(OsError::Message(
                    "could not initialize xcb_cursor",
                )));
            }

            Rc::new(AppState {
                connection,
                screen,
                atoms,
                shm_supported,
                cursor_context,
                cursor_cache: RefCell::new(HashMap::new()),
                running: Cell::new(false),
                windows: RefCell::new(HashMap::new()),
            })
        };

        let cx = AppContext::from_inner(AppContextInner::new(&state));
        let data = build(&cx)?;

        let inner = AppInner {
            state,
            data: Box::new(data),
        };

        Ok(inner)
    }

    pub fn run(&mut self) -> Result<()> {
        self.state.running.set(true);

        while self.state.running.get() {
            unsafe {
                let event = xcb::xcb_wait_for_event(self.state.connection);
                if event.is_null() {
                    let error = xcb::xcb_connection_has_error(self.state.connection);
                    return Err(Error::Os(OsError::Xcb(error)));
                }

                self.handle_event(event);

                libc::free(event as *mut c_void);
            }
        }

        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        loop {
            unsafe {
                let event = xcb::xcb_poll_for_event(self.state.connection);
                if event.is_null() {
                    break;
                }

                self.handle_event(event);

                libc::free(event as *mut c_void);
            }
        }

        Ok(())
    }

    unsafe fn handle_event(&mut self, event: *mut xcb::xcb_generic_event_t) {
        match ((*event).response_type & !0x80) as u32 {
            xcb::XCB_EXPOSE => {
                let event = &*(event as *mut xcb_sys::xcb_expose_event_t);
                let window = self.state.windows.borrow().get(&event.window).cloned();
                if let Some(window) = window {
                    window.expose_rects.borrow_mut().push(Rect {
                        x: event.x as f64,
                        y: event.y as f64,
                        width: event.width as f64,
                        height: event.height as f64,
                    });

                    if event.count == 0 {
                        let rects = window.expose_rects.take();
                        window.handler.borrow_mut()(
                            &mut *self.data,
                            &self.state,
                            Event::Expose(&rects),
                        );
                    }
                }
            }
            xcb::XCB_CLIENT_MESSAGE => {
                let event = &*(event as *mut xcb::xcb_client_message_event_t);
                if event.data.data32[0] == self.state.atoms.wm_delete_window {
                    let window = self.state.windows.borrow().get(&event.window).cloned();
                    if let Some(window) = window {
                        window.handler.borrow_mut()(&mut *self.data, &self.state, Event::Close);
                    }
                }
            }
            xcb::XCB_MOTION_NOTIFY => {
                let event = &*(event as *mut xcb_sys::xcb_motion_notify_event_t);
                let window = self.state.windows.borrow().get(&event.event).cloned();
                if let Some(window) = window {
                    let point = Point {
                        x: event.event_x as f64,
                        y: event.event_y as f64,
                    };

                    window.handler.borrow_mut()(
                        &mut *self.data,
                        &self.state,
                        Event::MouseMove(point),
                    );
                }
            }
            xcb::XCB_BUTTON_PRESS => {
                let event = &*(event as *mut xcb_sys::xcb_button_press_event_t);
                let window = self.state.windows.borrow().get(&event.event).cloned();
                if let Some(window) = window {
                    if let Some(button) = mouse_button_from_code(event.detail) {
                        window.handler.borrow_mut()(
                            &mut *self.data,
                            &self.state,
                            Event::MouseDown(button),
                        );
                    } else if let Some(delta) = scroll_delta_from_code(event.detail) {
                        window.handler.borrow_mut()(
                            &mut *self.data,
                            &self.state,
                            Event::Scroll(delta),
                        );
                    }
                }
            }
            xcb::XCB_BUTTON_RELEASE => {
                let event = &*(event as *mut xcb_sys::xcb_button_release_event_t);
                let window = self.state.windows.borrow().get(&event.event).cloned();
                if let Some(window) = window {
                    if let Some(button) = mouse_button_from_code(event.detail) {
                        window.handler.borrow_mut()(
                            &mut *self.data,
                            &self.state,
                            Event::MouseUp(button),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>> {
        Ok(*self.data)
    }
}

impl<T> AsRawFd for AppInner<T> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { xcb::xcb_get_file_descriptor(self.state.connection) }
    }
}

pub struct AppContextInner<'a, T> {
    pub state: &'a Rc<AppState>,
    _marker: PhantomData<T>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub(super) fn new(state: &'a Rc<AppState>) -> AppContextInner<'a, T> {
        AppContextInner {
            state,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerHandleInner
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        TimerHandleInner {}
    }

    pub fn exit(&self) {
        self.state.running.set(false);
    }
}
