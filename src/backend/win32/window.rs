use std::alloc::{alloc, dealloc, Layout};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, c_void};
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::{mem, ptr, slice};

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{
    GetLastError, FALSE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{self as gdi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    self as msg, AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GetWindowLongPtrW, LoadCursorW, SetCursor, SetCursorPos, SetWindowLongPtrW, ShowWindow,
};

use super::app::{AppContextInner, AppState};
use super::{hinstance, to_wstring, OsError};
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, MouseButton, Point, RawParent, Rect, Response,
    Result, Size, WindowOptions,
};

#[allow(non_snake_case)]
fn LOWORD(l: u32) -> u16 {
    (l & 0xffff) as u16
}

#[allow(non_snake_case)]
fn HIWORD(l: u32) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

#[allow(non_snake_case)]
fn GET_X_LPARAM(lp: LPARAM) -> i16 {
    LOWORD(lp as u32) as i16
}

#[allow(non_snake_case)]
fn GET_Y_LPARAM(lp: LPARAM) -> i16 {
    HIWORD(lp as u32) as i16
}

#[allow(non_snake_case)]
fn GET_XBUTTON_WPARAM(wParam: WPARAM) -> u16 {
    HIWORD(wParam as u32)
}

#[allow(non_snake_case)]
fn GET_WHEEL_DELTA_WPARAM(wParam: WPARAM) -> i16 {
    HIWORD(wParam as u32) as i16
}

struct WindowState {
    mouse_down_count: Cell<isize>,
    cursor: Cell<Cursor>,
    app_state: Rc<AppState>,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>, Event) -> Response>>,
}

impl WindowState {
    fn update_cursor(&self) {
        unsafe {
            let hcursor = match self.cursor.get() {
                Cursor::Arrow => LoadCursorW(0, msg::IDC_ARROW),
                Cursor::Crosshair => LoadCursorW(0, msg::IDC_CROSS),
                Cursor::Hand => LoadCursorW(0, msg::IDC_HAND),
                Cursor::IBeam => LoadCursorW(0, msg::IDC_IBEAM),
                Cursor::No => LoadCursorW(0, msg::IDC_NO),
                Cursor::SizeNs => LoadCursorW(0, msg::IDC_SIZENS),
                Cursor::SizeWe => LoadCursorW(0, msg::IDC_SIZEWE),
                Cursor::SizeNesw => LoadCursorW(0, msg::IDC_SIZENESW),
                Cursor::SizeNwse => LoadCursorW(0, msg::IDC_SIZENWSE),
                Cursor::Wait => LoadCursorW(0, msg::IDC_WAIT),
                Cursor::None => 0,
            };

            SetCursor(hcursor);
        }
    }

    fn handle_event(&self, event: Event) -> Option<Response> {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            if let Ok(mut data) = self.app_state.data.try_borrow_mut() {
                if let Some(data) = &mut *data {
                    return Some(handler(&mut **data, &self.app_state, event));
                }
            }
        }

        None
    }
}

pub struct WindowInner {
    hwnd: HWND,
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
            let window_name = to_wstring(&options.title);

            let mut style = msg::WS_CLIPCHILDREN | msg::WS_CLIPSIBLINGS;

            if options.parent.is_some() {
                style |= msg::WS_CHILD;
            } else {
                style |= msg::WS_CAPTION
                    | msg::WS_SIZEBOX
                    | msg::WS_SYSMENU
                    | msg::WS_MINIMIZEBOX
                    | msg::WS_MAXIMIZEBOX;
            }

            let position = options.position.unwrap_or(Point::new(0.0, 0.0));

            let mut rect = RECT {
                left: position.x.round() as i32,
                top: position.y.round() as i32,
                right: (position.x + options.size.width).round() as i32,
                bottom: (position.y + options.size.height).round() as i32,
            };
            AdjustWindowRectEx(&mut rect, style, FALSE, 0);

            let parent = if let Some(parent) = options.parent {
                if let RawParent::Win32(hwnd) = parent {
                    hwnd as HWND
                } else {
                    return Err(Error::InvalidWindowHandle);
                }
            } else {
                0
            };

            let (x, y) = if options.position.is_some() {
                (rect.top, rect.left)
            } else {
                (msg::CW_USEDEFAULT, msg::CW_USEDEFAULT)
            };

            let hwnd = CreateWindowExW(
                0,
                cx.inner.state.class as PCWSTR,
                window_name.as_ptr(),
                style,
                x,
                y,
                rect.right - rect.left,
                rect.bottom - rect.top,
                parent,
                0,
                hinstance(),
                ptr::null_mut(),
            );
            if hwnd == 0 {
                return Err(Error::Os(OsError {
                    code: GetLastError(),
                }));
            }

            let mut handler = handler;
            let handler_wrapper =
                move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                    let data = data_any.downcast_mut::<T>().unwrap();
                    let cx = AppContext::from_inner(AppContextInner::new(app_state));
                    handler(data, &cx, event)
                };

            let state = Rc::new(WindowState {
                mouse_down_count: Cell::new(0),
                cursor: Cell::new(Cursor::Arrow),
                app_state: Rc::clone(cx.inner.state),
                handler: RefCell::new(Box::new(handler_wrapper)),
            });

            let state_ptr = Rc::into_raw(Rc::clone(&state));
            SetWindowLongPtrW(hwnd, msg::GWLP_USERDATA, state_ptr as isize);

            Ok(WindowInner { hwnd, state })
        }
    }

    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, msg::SW_SHOWNORMAL);
        }
    }

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, msg::SW_HIDE);
        }
    }

    pub fn size(&self) -> Size {
        unimplemented!()
    }

    pub fn scale(&self) -> f64 {
        unimplemented!()
    }

    pub fn present(&self, bitmap: Bitmap) {
        self.present_inner(bitmap, None);
    }

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {
        self.present_inner(bitmap, Some(rects));
    }

    fn present_inner(&self, bitmap: Bitmap, rects: Option<&[Rect]>) {
        unsafe {
            let hdc = gdi::GetDC(self.hwnd);
            if hdc != 0 {
                if let Some(rects) = rects {
                    let (layout, _) = Layout::new::<gdi::RGNDATAHEADER>()
                        .extend(Layout::array::<RECT>(rects.len()).unwrap())
                        .unwrap();
                    let ptr = alloc(layout) as *mut gdi::RGNDATA;

                    let buffer_ptr = ptr::addr_of!((*ptr).Buffer) as *mut MaybeUninit<RECT>;
                    let buffer = slice::from_raw_parts_mut(buffer_ptr, rects.len());
                    for (src, dst) in rects.iter().zip(buffer.iter_mut()) {
                        dst.write(RECT {
                            left: src.x.round() as i32,
                            top: src.y.round() as i32,
                            right: (src.x + src.width).round() as i32,
                            bottom: (src.y + src.height).round() as i32,
                        });
                    }

                    let buffer = slice::from_raw_parts(buffer_ptr as *const RECT, rects.len());
                    let bounds = if buffer.is_empty() {
                        RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        }
                    } else {
                        let mut bounds = buffer[0];
                        for rect in buffer {
                            bounds.left = bounds.left.min(rect.left);
                            bounds.top = bounds.top.min(rect.top);
                            bounds.right = bounds.right.max(rect.right);
                            bounds.bottom = bounds.bottom.max(rect.bottom);
                        }
                        bounds
                    };

                    (*ptr).rdh = gdi::RGNDATAHEADER {
                        dwSize: mem::size_of::<gdi::RGNDATAHEADER>() as u32,
                        iType: gdi::RDH_RECTANGLES,
                        nCount: rects.len() as u32,
                        nRgnSize: layout.size() as u32,
                        rcBound: bounds,
                    };

                    let rgn = gdi::ExtCreateRegion(ptr::null(), layout.size() as u32, ptr);
                    gdi::SelectClipRgn(hdc, rgn);
                    gdi::DeleteObject(rgn as gdi::HGDIOBJ);

                    dealloc(ptr as *mut u8, layout);
                }

                let bitmap_info = gdi::BITMAPINFO {
                    bmiHeader: gdi::BITMAPINFOHEADER {
                        biSize: mem::size_of::<gdi::BITMAPINFOHEADER>() as u32,
                        biWidth: bitmap.width() as i32,
                        biHeight: -(bitmap.height() as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: gdi::BI_RGB as u32,
                        ..mem::zeroed()
                    },
                    ..mem::zeroed()
                };

                gdi::StretchDIBits(
                    hdc,
                    0,
                    0,
                    bitmap.width() as i32,
                    bitmap.height() as i32,
                    0,
                    0,
                    bitmap.width() as i32,
                    bitmap.height() as i32,
                    bitmap.data().as_ptr() as *const c_void,
                    &bitmap_info,
                    gdi::DIB_RGB_COLORS,
                    gdi::SRCCOPY,
                );

                if rects.is_some() {
                    gdi::SelectClipRgn(hdc, 0);
                }

                gdi::ReleaseDC(self.hwnd, hdc);
            }
        }
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        self.state.cursor.set(cursor);
        self.state.update_cursor();
    }

    pub fn set_mouse_position(&self, position: Point) {
        unsafe {
            let mut point = POINT {
                x: position.x as c_int,
                y: position.y as c_int,
            };
            gdi::ClientToScreen(self.hwnd, &mut point);
            SetCursorPos(point.x, point.y);
        }
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, msg::GWLP_USERDATA) as *const WindowState;
    if !state_ptr.is_null() {
        // Hold a reference to the WindowState for the duration of the wnd_proc, in case the
        // window is closed during an event handler
        let state_rc = Rc::from_raw(state_ptr);
        let state = Rc::clone(&state_rc);
        let _ = Rc::into_raw(state_rc);

        match msg {
            msg::WM_SETCURSOR => {
                if LOWORD(lparam as u32) == msg::HTCLIENT as u16 {
                    state.update_cursor();
                    return 0;
                }
            }
            msg::WM_ERASEBKGND => {
                return 1;
            }
            msg::WM_PAINT => {
                let mut rects = Vec::new();

                let rgn = gdi::CreateRectRgn(0, 0, 0, 0);
                gdi::GetUpdateRgn(hwnd, rgn, 0);
                let size = gdi::GetRegionData(rgn, 0, ptr::null_mut());
                if size != 0 {
                    let align = mem::align_of::<gdi::RGNDATA>();
                    let layout = Layout::from_size_align(size as usize, align).unwrap();
                    let ptr = alloc(layout) as *mut gdi::RGNDATA;

                    let result = gdi::GetRegionData(rgn, size, ptr);
                    if result == size {
                        let count = (*ptr).rdh.nCount as usize;

                        let buffer_ptr = ptr::addr_of!((*ptr).Buffer) as *const RECT;
                        let buffer = slice::from_raw_parts(buffer_ptr, count);

                        rects.reserve_exact(count);
                        for rect in buffer {
                            rects.push(Rect {
                                x: rect.left as f64,
                                y: rect.top as f64,
                                width: (rect.right - rect.left) as f64,
                                height: (rect.bottom - rect.top) as f64,
                            });
                        }
                    }

                    dealloc(ptr as *mut u8, layout);
                }
                gdi::DeleteObject(rgn as gdi::HGDIOBJ);

                // Only validate the dirty region if we successfully invoked the event handler.
                // This ensures that if we receive an expose event during the App::new builder
                // callback, we will receive it again later.
                if state.handle_event(Event::Expose(&rects)).is_some() {
                    gdi::ValidateRgn(hwnd, 0);
                }

                return 0;
            }
            msg::WM_MOUSEMOVE => {
                let point = Point {
                    x: GET_X_LPARAM(lparam) as f64,
                    y: GET_Y_LPARAM(lparam) as f64,
                };
                state.handle_event(Event::MouseMove(point));

                return 0;
            }
            msg::WM_LBUTTONDOWN
            | msg::WM_LBUTTONUP
            | msg::WM_MBUTTONDOWN
            | msg::WM_MBUTTONUP
            | msg::WM_RBUTTONDOWN
            | msg::WM_RBUTTONUP
            | msg::WM_XBUTTONDOWN
            | msg::WM_XBUTTONUP => {
                let button = match msg {
                    msg::WM_LBUTTONDOWN | msg::WM_LBUTTONUP => Some(MouseButton::Left),
                    msg::WM_MBUTTONDOWN | msg::WM_MBUTTONUP => Some(MouseButton::Middle),
                    msg::WM_RBUTTONDOWN | msg::WM_RBUTTONUP => Some(MouseButton::Right),
                    msg::WM_XBUTTONDOWN | msg::WM_XBUTTONUP => match GET_XBUTTON_WPARAM(wparam) {
                        msg::XBUTTON1 => Some(MouseButton::Back),
                        msg::XBUTTON2 => Some(MouseButton::Forward),
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(button) = button {
                    let event = match msg {
                        msg::WM_LBUTTONDOWN
                        | msg::WM_MBUTTONDOWN
                        | msg::WM_RBUTTONDOWN
                        | msg::WM_XBUTTONDOWN => Some(Event::MouseDown(button)),
                        msg::WM_LBUTTONUP
                        | msg::WM_MBUTTONUP
                        | msg::WM_RBUTTONUP
                        | msg::WM_XBUTTONUP => Some(Event::MouseUp(button)),
                        _ => None,
                    };

                    if let Some(event) = event {
                        match event {
                            Event::MouseDown(_) => {
                                state.mouse_down_count.set(state.mouse_down_count.get() + 1);
                                if state.mouse_down_count.get() == 1 {
                                    SetCapture(hwnd);
                                }
                            }
                            Event::MouseUp(_) => {
                                state.mouse_down_count.set(state.mouse_down_count.get() - 1);
                                if state.mouse_down_count.get() == 0 {
                                    ReleaseCapture();
                                }
                            }
                            _ => {}
                        }

                        if state.handle_event(event) == Some(Response::Capture) {
                            return 0;
                        }
                    }
                }
            }
            msg::WM_MOUSEWHEEL | msg::WM_MOUSEHWHEEL => {
                let delta = GET_WHEEL_DELTA_WPARAM(wparam) as f64 / 120.0;
                let point = match msg {
                    msg::WM_MOUSEWHEEL => Point::new(0.0, delta),
                    msg::WM_MOUSEHWHEEL => Point::new(delta, 0.0),
                    _ => unreachable!(),
                };

                if state.handle_event(Event::Scroll(point)) == Some(Response::Capture) {
                    return 0;
                }
            }
            msg::WM_CLOSE => {
                state.handle_event(Event::Close);
                return 0;
            }
            msg::WM_DESTROY => {
                drop(Rc::from_raw(state_ptr));
                SetWindowLongPtrW(hwnd, msg::GWLP_USERDATA, 0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
