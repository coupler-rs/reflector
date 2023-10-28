use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, c_void};
use std::rc::Rc;
use std::{mem, ptr, slice};

use x11rb::connection::Connection;
use x11rb::protocol::shm::{ConnectionExt as _, Seg};
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ClipOrdering, ConnectionExt as _, CreateGCAux,
    CreateWindowAux, EventMask, Gcontext, ImageFormat, PropMode, Rectangle, Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use super::app::{AppContextInner, AppState};
use super::OsError;
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, Point, RawParent, Rect, Response, Result, Size,
    WindowOptions,
};

pub struct ShmState {
    shm_id: c_int,
    seg_id: Seg,
    ptr: *mut c_void,
    width: usize,
    height: usize,
}

pub struct WindowState {
    pub window_id: Cell<Option<Window>>,
    pub gc_id: Cell<Option<Gcontext>>,
    pub shm_state: RefCell<Option<ShmState>>,
    pub expose_rects: RefCell<Vec<Rect>>,
    pub app_state: Rc<AppState>,
    pub handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>, Event) -> Response>>,
}

impl WindowState {
    fn init_shm(app_state: &AppState, width: usize, height: usize) -> Result<Option<ShmState>> {
        if !app_state.shm_supported {
            return Ok(None);
        }

        let shm_id = unsafe {
            let shm_id = libc::shmget(
                libc::IPC_PRIVATE,
                width * height * mem::size_of::<u32>(),
                libc::IPC_CREAT | 0o600,
            );
            if shm_id == -1 {
                return Err(Error::Os(OsError::Message(
                    "failed to create shared memory segment",
                )));
            }

            shm_id
        };

        let ptr = unsafe {
            let ptr = libc::shmat(shm_id, ptr::null(), 0);
            if ptr == usize::MAX as *mut c_void {
                libc::shmctl(shm_id, libc::IPC_RMID, ptr::null_mut());
                return Err(Error::Os(OsError::Message(
                    "failed to attach shared memory segment",
                )));
            }

            ptr
        };

        let seg_id = app_state.connection.generate_id()?;
        app_state.connection.shm_attach(seg_id, shm_id as u32, false)?;

        Ok(Some(ShmState {
            shm_id,
            seg_id,
            ptr,
            width,
            height,
        }))
    }

    fn deinit_shm(&self) {
        if let Some(shm_state) = self.shm_state.take() {
            let _ = self.app_state.connection.shm_detach(shm_state.seg_id);

            unsafe {
                libc::shmdt(shm_state.ptr);
                libc::shmctl(shm_state.shm_id, libc::IPC_RMID, ptr::null_mut());
            }
        }
    }

    pub fn close(&self) {
        if let Some(window_id) = self.window_id.take() {
            self.app_state.windows.borrow_mut().remove(&window_id);
            let _ = self.app_state.connection.destroy_window(window_id);
        }

        if let Some(gc_id) = self.gc_id.take() {
            let _ = self.app_state.connection.free_gc(gc_id);
        }

        self.deinit_shm();
    }
}

#[derive(Clone)]
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
        let connection = &cx.inner.state.connection;

        let window_id = connection.generate_id()?;

        let parent_id = if let Some(parent) = options.parent {
            if let RawParent::X11(parent_id) = parent {
                parent_id as Window
            } else {
                return Err(Error::InvalidWindowHandle);
            }
        } else {
            connection.setup().roots[cx.inner.state.screen_index].root
        };

        let position = options.position.unwrap_or(Point::new(0.0, 0.0));

        let event_mask = EventMask::EXPOSURE
            | EventMask::POINTER_MOTION
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE;
        let aux = CreateWindowAux::new().event_mask(event_mask);

        connection.create_window(
            x11rb::COPY_FROM_PARENT as u8,
            window_id,
            parent_id,
            position.x.round() as i16,
            position.y.round() as i16,
            options.size.width.round() as u16,
            options.size.height.round() as u16,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &aux,
        )?;

        connection.change_property8(
            PropMode::REPLACE,
            window_id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            options.title.as_bytes(),
        )?;
        connection.change_property8(
            PropMode::REPLACE,
            window_id,
            cx.inner.state.atoms._NET_WM_NAME,
            cx.inner.state.atoms.UTF8_STRING,
            options.title.as_bytes(),
        )?;
        connection.change_property32(
            PropMode::REPLACE,
            window_id,
            cx.inner.state.atoms.WM_PROTOCOLS,
            AtomEnum::ATOM,
            &[cx.inner.state.atoms.WM_DELETE_WINDOW],
        )?;

        let gc_id = connection.generate_id()?;
        connection.create_gc(gc_id, window_id, &CreateGCAux::default())?;

        let shm_state = WindowState::init_shm(
            &cx.inner.state,
            options.size.width.round() as usize,
            options.size.height.round() as usize,
        )?;

        connection.flush()?;

        let mut handler = handler;
        let handler_wrapper =
            move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                let data = data_any.downcast_mut::<T>().unwrap();
                let cx = AppContext::from_inner(AppContextInner::new(app_state));
                handler(data, &cx, event)
            };

        let state = Rc::new(WindowState {
            window_id: Cell::new(Some(window_id)),
            gc_id: Cell::new(Some(gc_id)),
            shm_state: RefCell::new(shm_state),
            expose_rects: RefCell::new(Vec::new()),
            app_state: Rc::clone(&cx.inner.state),
            handler: RefCell::new(Box::new(handler_wrapper)),
        });

        cx.inner.state.windows.borrow_mut().insert(window_id, Rc::clone(&state));

        Ok(WindowInner { state })
    }

    pub fn show(&self) {
        if let Some(window_id) = self.state.window_id.get() {
            let _ = self.state.app_state.connection.map_window(window_id);
            let _ = self.state.app_state.connection.flush();
        }
    }

    pub fn hide(&self) {
        if let Some(window_id) = self.state.window_id.get() {
            let _ = self.state.app_state.connection.unmap_window(window_id);
            let _ = self.state.app_state.connection.flush();
        }
    }

    pub fn size(&self) -> Size {
        self.size_inner().unwrap_or(Size::new(0.0, 0.0))
    }

    fn size_inner(&self) -> Result<Size> {
        let window_id = self.state.window_id.get().ok_or(Error::WindowClosed)?;
        let geom = self.state.app_state.connection.get_geometry(window_id)?.reply()?;

        Ok(Size::new(geom.width as f64, geom.height as f64))
    }

    pub fn scale(&self) -> f64 {
        self.state.app_state.scale
    }

    pub fn present(&self, bitmap: Bitmap) {
        let _ = self.present_inner(bitmap, None);
    }

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {
        let _ = self.present_inner(bitmap, Some(rects));
    }

    fn present_inner(&self, bitmap: Bitmap, rects: Option<&[Rect]>) -> Result<()> {
        let connection = &self.state.app_state.connection;
        let window_id = self.state.window_id.get().ok_or(Error::WindowClosed)?;
        let gc_id = self.state.gc_id.get().ok_or(Error::WindowClosed)?;

        if let Some(rects) = rects {
            let mut x_rects = Vec::with_capacity(rects.len());
            for rect in rects {
                x_rects.push(Rectangle {
                    x: rect.x.round() as i16,
                    y: rect.y.round() as i16,
                    width: rect.width.round() as u16,
                    height: rect.height.round() as u16,
                });
            }

            connection.set_clip_rectangles(ClipOrdering::UNSORTED, gc_id, 0, 0, &x_rects)?;
        }

        if let Some(ref shm_state) = *self.state.shm_state.borrow() {
            // SAFETY: ptr is page-aligned and thus u32-aligned
            let data = unsafe {
                slice::from_raw_parts_mut(
                    shm_state.ptr as *mut u32,
                    shm_state.width * shm_state.height * std::mem::size_of::<u32>(),
                )
            };

            let copy_width = bitmap.width().min(shm_state.width);
            let copy_height = bitmap.height().min(shm_state.height);
            for row in 0..copy_height {
                let src = &bitmap.data()[row * bitmap.width()..row * bitmap.width() + copy_width];
                let dst = &mut data[row * shm_state.width..row * shm_state.width + copy_width];
                dst.copy_from_slice(src);
            }

            connection.shm_put_image(
                window_id,
                gc_id,
                shm_state.width as u16,
                shm_state.height as u16,
                0,
                0,
                shm_state.width as u16,
                shm_state.height as u16,
                0,
                0,
                24,
                ImageFormat::Z_PIXMAP.into(),
                false,
                shm_state.seg_id,
                0,
            )?;
        } else {
            let (_, bytes, _) = unsafe { bitmap.data().align_to::<u8>() };
            connection.put_image(
                ImageFormat::Z_PIXMAP,
                window_id,
                gc_id,
                bitmap.width() as u16,
                bitmap.height() as u16,
                0,
                0,
                0,
                24,
                bytes,
            )?;
        }

        if rects.is_some() {
            connection.set_clip_rectangles(ClipOrdering::UNSORTED, gc_id, 0, 0, &[])?;
        }

        connection.flush()?;

        Ok(())
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        let _ = self.set_cursor_inner(cursor);
    }

    fn set_cursor_inner(&self, cursor: Cursor) -> Result<()> {
        let connection = &self.state.app_state.connection;
        let cursor_cache = &self.state.app_state.cursor_cache;
        let window_id = self.state.window_id.get().ok_or(Error::WindowClosed)?;

        let cursor_id = if let Some(cursor_id) = cursor_cache.borrow_mut().get(&cursor) {
            *cursor_id
        } else {
            if cursor == Cursor::None {
                let cursor_id = connection.generate_id()?;
                let pixmap_id = connection.generate_id()?;
                let root = connection.setup().roots[self.state.app_state.screen_index].root;
                connection.create_pixmap(1, pixmap_id, root, 1, 1)?;
                connection
                    .create_cursor(cursor_id, pixmap_id, pixmap_id, 0, 0, 0, 0, 0, 0, 0, 0)?;
                connection.free_pixmap(pixmap_id)?;

                cursor_id
            } else {
                let cursor_name = match cursor {
                    Cursor::Arrow => "left_ptr",
                    Cursor::Crosshair => "crosshair",
                    Cursor::Hand => "hand2",
                    Cursor::IBeam => "text",
                    Cursor::No => "crossed_circle",
                    Cursor::SizeNs => "v_double_arrow",
                    Cursor::SizeWe => "h_double_arrow",
                    Cursor::SizeNesw => "fd_double_arrow",
                    Cursor::SizeNwse => "bd_double_arrow",
                    Cursor::Wait => "watch",
                    Cursor::None => unreachable!(),
                };
                self.state.app_state.cursor_handle.load_cursor(connection, cursor_name)?
            }
        };

        connection.change_window_attributes(
            window_id,
            &ChangeWindowAttributesAux::new().cursor(cursor_id),
        )?;
        self.state.app_state.connection.flush()?;

        Ok(())
    }

    pub fn set_mouse_position(&self, position: Point) {
        if let Some(window_id) = self.state.window_id.get() {
            let _ = self.state.app_state.connection.warp_pointer(
                x11rb::NONE,
                window_id,
                0,
                0,
                0,
                0,
                position.x.round() as i16,
                position.y.round() as i16,
            );
            let _ = self.state.app_state.connection.flush();
        }
    }

    pub fn close(&self) {
        self.state.close();
        let _ = self.state.app_state.connection.flush();
    }
}
