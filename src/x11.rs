use crate::{
    App, AppContext, Bitmap, CloseError, Cursor, Error, Event, Point, Rect, Response, Result,
    Window, WindowOptions,
};

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::rc::Rc;
use std::time::Duration;
use std::{fmt, mem, ptr, result};

use raw_window_handle::{unix::XcbHandle, RawWindowHandle};
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

pub struct TimerHandleInner {}

impl TimerHandleInner {
    pub fn cancel(self) {}
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
    _net_wm_name: xcb::xcb_atom_t,
    utf8_string: xcb::xcb_atom_t,
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

trait RemoveWindow {
    fn remove_window(&self, window_id: xcb::xcb_window_t);
}

type Handler<T> = dyn FnMut(&mut T, &AppContext<T>, Event) -> Response;

struct Windows<T>(RefCell<HashMap<xcb::xcb_window_t, Rc<WindowState<Handler<T>>>>>);

impl<T> RemoveWindow for Windows<T> {
    fn remove_window(&self, window_id: xcb::xcb_window_t) {
        self.0.borrow_mut().remove(&window_id);
    }
}

struct AppState<W: ?Sized> {
    connection: *mut xcb::xcb_connection_t,
    screen: *mut xcb::xcb_screen_t,
    atoms: Atoms,
    running: Cell<bool>,
    windows: W,
}

impl<H: ?Sized> Drop for AppState<H> {
    fn drop(&mut self) {
        unsafe {
            xcb::xcb_disconnect(self.connection);
        }
    }
}

pub struct AppInner<T> {
    state: Rc<AppState<Windows<T>>>,
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
                running: Cell::new(false),
                windows: Windows(RefCell::new(HashMap::new())),
            })
        };

        let cx = AppContext::from_inner(AppContextInner { state: &state });
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
                    return Err(Error::Os(OsError { code: error }));
                }

                self.handle_event(event);

                libc::free(event as *mut c_void);
            }
        }

        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        Ok(())
    }

    unsafe fn handle_event(&mut self, event: *mut xcb::xcb_generic_event_t) {
        match ((*event).response_type & !0x80) as u32 {
            xcb::XCB_EXPOSE => {
                let event = &*(event as *mut xcb_sys::xcb_expose_event_t);
                let window = self.state.windows.0.borrow().get(&event.window).cloned();
                if let Some(window) = window {
                    window.expose_rects.borrow_mut().push(Rect {
                        x: event.x as f64,
                        y: event.y as f64,
                        width: event.width as f64,
                        height: event.height as f64,
                    });

                    if event.count == 0 {
                        let rects = window.expose_rects.take();

                        let cx = AppContext::from_inner(AppContextInner { state: &self.state });
                        window.handler.borrow_mut()(&mut self.data, &cx, Event::Expose(&rects));
                    }
                }
            }
            xcb::XCB_CLIENT_MESSAGE => {
                let event = &*(event as *mut xcb::xcb_client_message_event_t);
                if event.data.data32[0] == self.state.atoms.wm_delete_window {
                    let window = self.state.windows.0.borrow().get(&event.window).cloned();
                    if let Some(window) = window {
                        let cx = AppContext::from_inner(AppContextInner { state: &self.state });
                        window.handler.borrow_mut()(&mut self.data, &cx, Event::Close);
                    }
                }
            }
            xcb::XCB_MOTION_NOTIFY => {
                let event = &*(event as *mut xcb_sys::xcb_motion_notify_event_t);
                let window = self.state.windows.0.borrow().get(&event.event).cloned();
                if let Some(window) = window {
                    let point = Point {
                        x: event.event_x as f64,
                        y: event.event_y as f64,
                    };

                    let cx = AppContext::from_inner(AppContextInner { state: &self.state });
                    window.handler.borrow_mut()(&mut self.data, &cx, Event::MouseMove(point));
                }
            }
            _ => {}
        }
    }

    pub fn into_inner(self) -> result::Result<T, CloseError<App<T>>> {
        Ok(*self.data)
    }
}

pub struct AppContextInner<'a, T> {
    state: &'a Rc<AppState<Windows<T>>>,
}

impl<'a, T> AppContextInner<'a, T> {
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

struct WindowState<H: ?Sized> {
    window_id: xcb::xcb_window_t,
    gc_id: xcb::xcb_gcontext_t,
    expose_rects: RefCell<Vec<Rect>>,
    app_state: Rc<AppState<dyn RemoveWindow>>,
    handler: RefCell<H>,
}

pub struct WindowInner {
    state: Rc<WindowState<dyn Any>>,
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
        let state = unsafe {
            let window_id = xcb::xcb_generate_id(cx.inner.state.connection);

            let parent_id = (*cx.inner.state.screen).root;

            let value_mask = xcb::XCB_CW_EVENT_MASK;
            let value_list = &[xcb::XCB_EVENT_MASK_EXPOSURE | xcb::XCB_EVENT_MASK_POINTER_MOTION];

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

            let title = options.title.as_bytes();
            xcb::xcb_change_property(
                cx.inner.state.connection,
                xcb::XCB_PROP_MODE_REPLACE as u8,
                window_id,
                cx.inner.state.atoms._net_wm_name,
                cx.inner.state.atoms.utf8_string,
                8,
                title.len() as u32,
                title.as_ptr() as *const c_void,
            );

            let gc_id = xcb::xcb_generate_id(cx.inner.state.connection);
            xcb::xcb_create_gc(cx.inner.state.connection, gc_id, window_id, 0, ptr::null());

            xcb::xcb_flush(cx.inner.state.connection);

            Rc::new(WindowState {
                window_id,
                gc_id,
                expose_rects: RefCell::new(Vec::new()),
                app_state: cx.inner.state.clone(),
                handler: RefCell::new(handler),
            })
        };

        let windows = &cx.inner.state.windows.0;
        windows.borrow_mut().insert(state.window_id, state.clone());

        Ok(WindowInner { state })
    }

    pub fn show(&self) {
        unsafe {
            xcb::xcb_map_window(self.state.app_state.connection, self.state.window_id);
            xcb::xcb_flush(self.state.app_state.connection);
        }
    }

    pub fn hide(&self) {
        unsafe {
            xcb::xcb_unmap_window(self.state.app_state.connection, self.state.window_id);
            xcb::xcb_flush(self.state.app_state.connection);
        }
    }

    pub fn present(&self, bitmap: Bitmap) {
        self.present_inner(bitmap, None);
    }

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {
        self.present_inner(bitmap, Some(rects));
    }

    fn present_inner(&self, bitmap: Bitmap, rects: Option<&[Rect]>) {
        unsafe {
            if let Some(rects) = rects {
                let mut xcb_rects = Vec::with_capacity(rects.len());
                for rect in rects {
                    xcb_rects.push(xcb::xcb_rectangle_t {
                        x: rect.x.round() as i16,
                        y: rect.y.round() as i16,
                        width: rect.width.round() as u16,
                        height: rect.height.round() as u16,
                    });
                }

                xcb::xcb_set_clip_rectangles(
                    self.state.app_state.connection,
                    xcb::XCB_CLIP_ORDERING_UNSORTED as u8,
                    self.state.gc_id,
                    0,
                    0,
                    xcb_rects.len() as u32,
                    xcb_rects.as_ptr(),
                );
            }

            xcb::xcb_put_image(
                self.state.app_state.connection,
                xcb::XCB_IMAGE_FORMAT_Z_PIXMAP as u8,
                self.state.window_id,
                self.state.gc_id,
                bitmap.width() as u16,
                bitmap.height() as u16,
                0,
                0,
                0,
                24,
                (bitmap.data().len() * mem::size_of::<u32>()) as u32,
                bitmap.data().as_ptr() as *const u8,
            );

            if rects.is_some() {
                xcb::xcb_set_clip_rectangles(
                    self.state.app_state.connection,
                    xcb::XCB_CLIP_ORDERING_UNSORTED as u8,
                    self.state.gc_id,
                    0,
                    0,
                    0,
                    ptr::null(),
                );
            }

            xcb::xcb_flush(self.state.app_state.connection);
        }
    }

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, position: Point) {
        unsafe {
            xcb::xcb_warp_pointer(
                self.state.app_state.connection,
                xcb::XCB_NONE,
                self.state.window_id,
                0,
                0,
                0,
                0,
                position.x as i16,
                position.y as i16,
            );
            xcb::xcb_flush(self.state.app_state.connection);
        }
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Xcb(XcbHandle {
            window: self.state.window_id,
            connection: self.state.app_state.connection as *mut c_void,
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
            let cookie = xcb::xcb_destroy_window_checked(
                self.state.app_state.connection,
                self.state.window_id,
            );
            let error = xcb::xcb_request_check(self.state.app_state.connection, cookie);

            if !error.is_null() {
                let error_code = (*error).error_code;
                libc::free(error as *mut c_void);
                return Err(Error::Os(OsError {
                    code: error_code as c_int,
                }));
            }
        }

        let windows = &self.state.app_state.windows;
        windows.remove_window(self.state.window_id);

        Ok(())
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}
