use std::any::Any;
use std::cell::RefCell;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::rc::Rc;
use std::{mem, ptr, slice};

use raw_window_handle::{unix::XcbHandle, RawWindowHandle};
use xcb_sys as xcb;

use super::app::{AppContextInner, AppState};
use super::OsError;
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, Point, Rect, Response, Result, WindowOptions,
};

pub struct ShmState {
    shm_id: c_int,
    shm_seg_id: xcb::xcb_shm_seg_t,
    shm_ptr: *mut c_void,
    width: usize,
    height: usize,
}

pub struct WindowState {
    pub window_id: xcb::xcb_window_t,
    pub gc_id: xcb::xcb_gcontext_t,
    pub shm_state: RefCell<Option<ShmState>>,
    pub expose_rects: RefCell<Vec<Rect>>,
    pub app_state: Rc<AppState>,
    pub handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>, Event) -> Response>>,
}

impl WindowState {
    unsafe fn init_shm(app_state: &AppState, width: usize, height: usize) -> Option<ShmState> {
        if !app_state.shm_supported {
            return None;
        }

        unsafe {
            let shm_id = libc::shmget(
                libc::IPC_PRIVATE,
                width * height * mem::size_of::<u32>(),
                libc::IPC_CREAT | 0o600,
            );
            if shm_id == -1 {
                return None;
            }

            let shm_ptr = libc::shmat(shm_id, ptr::null(), 0);
            if shm_ptr == usize::MAX as *mut c_void {
                libc::shmctl(shm_id, libc::IPC_RMID, ptr::null_mut());
                return None;
            }

            let shm_seg_id = xcb::xcb_generate_id(app_state.connection);
            let cookie =
                xcb::xcb_shm_attach_checked(app_state.connection, shm_seg_id, shm_id as u32, 0);
            let error = xcb::xcb_request_check(app_state.connection, cookie);
            if !error.is_null() {
                libc::free(error as *mut c_void);
                libc::shmdt(shm_ptr);
                libc::shmctl(shm_id, libc::IPC_RMID, ptr::null_mut());
                return None;
            }

            Some(ShmState {
                shm_id,
                shm_seg_id,
                shm_ptr,
                width,
                height,
            })
        }
    }

    unsafe fn deinit_shm(&self) {
        if let Some(shm_state) = self.shm_state.take() {
            xcb::xcb_shm_detach(self.app_state.connection, shm_state.shm_seg_id);
            libc::shmdt(shm_state.shm_ptr);
            libc::shmctl(shm_state.shm_id, libc::IPC_RMID, ptr::null_mut());
        }
    }
}

impl Drop for WindowState {
    fn drop(&mut self) {
        unsafe {
            self.deinit_shm();

            xcb::xcb_destroy_window(self.app_state.connection, self.window_id);
            xcb::xcb_flush(self.app_state.connection);
        }
    }
}

pub struct WindowInner {
    state: Rc<WindowState>,
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
        unsafe {
            let window_id = xcb::xcb_generate_id(cx.inner.state.connection);

            let parent_id = (*cx.inner.state.screen).root;

            let value_mask = xcb::XCB_CW_EVENT_MASK;
            let value_list = &[xcb::XCB_EVENT_MASK_EXPOSURE
                | xcb::XCB_EVENT_MASK_POINTER_MOTION
                | xcb::XCB_EVENT_MASK_BUTTON_PRESS
                | xcb::XCB_EVENT_MASK_BUTTON_RELEASE];

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
                return Err(Error::Os(OsError::Xcb(error_code as c_int)));
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

            let shm_state = WindowState::init_shm(
                &cx.inner.state,
                options.rect.width as usize,
                options.rect.height as usize,
            );

            xcb::xcb_flush(cx.inner.state.connection);

            let mut handler = handler;
            let handler_wrapper =
                move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                    let data = data_any.downcast_mut::<T>().unwrap();
                    let cx = AppContext::from_inner(AppContextInner::new(app_state));
                    handler(data, &cx, event)
                };

            let state = Rc::new(WindowState {
                window_id,
                gc_id,
                shm_state: RefCell::new(shm_state),
                expose_rects: RefCell::new(Vec::new()),
                app_state: Rc::clone(&cx.inner.state),
                handler: RefCell::new(Box::new(handler_wrapper)),
            });

            cx.inner.state.windows.borrow_mut().insert(window_id, Rc::clone(&state));

            Ok(WindowInner { state })
        }
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

            if let Some(ref shm_state) = *self.state.shm_state.borrow() {
                // This is safe because shm_ptr is page-aligned and thus u32-aligned
                let data = slice::from_raw_parts_mut(
                    shm_state.shm_ptr as *mut u32,
                    shm_state.width * shm_state.height * std::mem::size_of::<u32>(),
                );

                let copy_width = bitmap.width().min(shm_state.width);
                let copy_height = bitmap.height().min(shm_state.height);
                for row in 0..copy_height {
                    let src =
                        &bitmap.data()[row * bitmap.width()..row * bitmap.width() + copy_width];
                    let dst = &mut data[row * shm_state.width..row * shm_state.width + copy_width];
                    dst.copy_from_slice(src);
                }

                let cookie = xcb::xcb_shm_put_image(
                    self.state.app_state.connection,
                    self.state.window_id,
                    self.state.gc_id,
                    shm_state.width as u16,
                    shm_state.height as u16,
                    0,
                    0,
                    shm_state.width as u16,
                    shm_state.height as u16,
                    0,
                    0,
                    24,
                    xcb::XCB_IMAGE_FORMAT_Z_PIXMAP as u8,
                    0,
                    shm_state.shm_seg_id,
                    0,
                );

                xcb::xcb_request_check(self.state.app_state.connection, cookie);
            } else {
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
            }

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

    pub fn set_cursor(&self, cursor: Cursor) {
        unsafe {
            let cursor_cache = &self.state.app_state.cursor_cache;
            let cursor_id = *cursor_cache.borrow_mut().entry(cursor).or_insert_with(|| {
                if cursor == Cursor::None {
                    let cursor_id = xcb::xcb_generate_id(self.state.app_state.connection);
                    let pixmap_id = xcb::xcb_generate_id(self.state.app_state.connection);
                    xcb::xcb_create_pixmap(
                        self.state.app_state.connection,
                        1,
                        pixmap_id,
                        (*self.state.app_state.screen).root,
                        1,
                        1,
                    );
                    xcb::xcb_create_cursor(
                        self.state.app_state.connection,
                        cursor_id,
                        pixmap_id,
                        pixmap_id,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    );
                    xcb::xcb_free_pixmap(self.state.app_state.connection, pixmap_id);
                    cursor_id
                } else {
                    let cursor_name = match cursor {
                        Cursor::Arrow => &b"left_ptr\0"[..],
                        Cursor::Crosshair => &b"crosshair\0"[..],
                        Cursor::Hand => &b"hand2\0"[..],
                        Cursor::IBeam => &b"text\0"[..],
                        Cursor::No => &b"crossed_circle\0"[..],
                        Cursor::SizeNs => &b"v_double_arrow\0"[..],
                        Cursor::SizeWe => &b"h_double_arrow\0"[..],
                        Cursor::SizeNesw => &b"fd_double_arrow\0"[..],
                        Cursor::SizeNwse => &b"bd_double_arrow\0"[..],
                        Cursor::Wait => &b"watch\0"[..],
                        Cursor::None => &b"\0"[..],
                    };
                    xcb::xcb_cursor_load_cursor(
                        self.state.app_state.cursor_context,
                        cursor_name.as_ptr() as *const c_char,
                    )
                }
            });

            xcb::xcb_change_window_attributes(
                self.state.app_state.connection,
                self.state.window_id,
                xcb::XCB_CW_CURSOR,
                &cursor_id as *const xcb::xcb_cursor_t as *const c_void,
            );

            xcb::xcb_flush(self.state.app_state.connection);
        }
    }

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
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        self.state.app_state.windows.borrow_mut().remove(&self.state.window_id);
    }
}
