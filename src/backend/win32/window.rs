use crate::{
    AppContext, Bitmap, Cursor, Error, Event, MouseButton, Parent, Point, Rect, Response, Result,
    WindowOptions,
};

use std::alloc::{alloc, dealloc, Layout};
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, c_void};
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::{mem, ptr, slice};

use raw_window_handle::{windows::WindowsHandle, RawWindowHandle};
use winapi::{
    shared::minwindef, shared::ntdef, shared::windef, shared::windowsx, um::errhandlingapi,
    um::wingdi, um::winuser,
};

use super::app::{AppContextInner, AppState};
use super::{hinstance, to_wstring, OsError};

pub trait HandleEvent {
    fn handle_event(&self, event: Event) -> Option<Response>;
}

pub struct Handler<T, H> {
    pub app_state: Rc<AppState<T>>,
    pub handler: RefCell<H>,
}

impl<T: 'static, H> HandleEvent for Handler<T, H>
where
    H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
{
    fn handle_event(&self, event: Event) -> Option<Response> {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            if let Ok(mut data) = self.app_state.data.try_borrow_mut() {
                if let Some(data) = data.as_mut() {
                    let cx = AppContext::from_inner(AppContextInner {
                        state: &self.app_state,
                    });
                    return Some(handler(data, &cx, event));
                }
            }
        }

        None
    }
}

struct WindowState<H: ?Sized> {
    hwnd: windef::HWND,
    mouse_down_count: Cell<isize>,
    cursor: Cell<Cursor>,
    handler: H,
}

impl WindowState<dyn HandleEvent> {
    fn update_cursor(&self) {
        unsafe {
            let hcursor = match self.cursor.get() {
                Cursor::Arrow => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_ARROW),
                Cursor::Crosshair => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_CROSS),
                Cursor::Hand => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_HAND),
                Cursor::IBeam => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_IBEAM),
                Cursor::No => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_NO),
                Cursor::SizeNs => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZENS),
                Cursor::SizeWe => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZEWE),
                Cursor::SizeNesw => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZENESW),
                Cursor::SizeNwse => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZENWSE),
                Cursor::Wait => winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_WAIT),
                Cursor::None => ptr::null_mut(),
            };

            winuser::SetCursor(hcursor);
        }
    }
}

pub struct WindowInner {
    state: Rc<WindowState<dyn HandleEvent>>,
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
            let window_name = to_wstring(&options.title);

            let mut style = winuser::WS_CLIPCHILDREN | winuser::WS_CLIPSIBLINGS;

            if let Some(Parent::Raw(_)) = options.parent {
                style |= winuser::WS_CHILD;
            } else {
                style |= winuser::WS_CAPTION
                    | winuser::WS_SIZEBOX
                    | winuser::WS_SYSMENU
                    | winuser::WS_MINIMIZEBOX
                    | winuser::WS_MAXIMIZEBOX;
            }

            let mut rect = windef::RECT {
                left: options.rect.x.round() as i32,
                top: options.rect.y.round() as i32,
                right: (options.rect.x + options.rect.width).round() as i32,
                bottom: (options.rect.y + options.rect.height).round() as i32,
            };
            winuser::AdjustWindowRectEx(&mut rect, style, minwindef::FALSE, 0);

            let parent = if let Some(Parent::Raw(parent)) = options.parent {
                if let RawWindowHandle::Windows(handle) = parent {
                    if !handle.hwnd.is_null() {
                        handle.hwnd as windef::HWND
                    } else {
                        return Err(Error::InvalidWindowHandle);
                    }
                } else {
                    return Err(Error::InvalidWindowHandle);
                }
            } else {
                ptr::null_mut()
            };

            let hwnd = winuser::CreateWindowExW(
                0,
                cx.inner.state.class as *const ntdef::WCHAR,
                window_name.as_ptr(),
                style,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                parent,
                ptr::null_mut(),
                hinstance(),
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Err(Error::Os(OsError {
                    code: errhandlingapi::GetLastError(),
                }));
            }

            let state = Rc::new(WindowState {
                hwnd,
                mouse_down_count: Cell::new(0),
                cursor: Cell::new(Cursor::Arrow),
                handler: Handler {
                    app_state: Rc::clone(cx.inner.state),
                    handler: RefCell::new(handler),
                },
            });

            // We can't store a wide pointer to the WindowState<dyn HandleEvent> in the window's
            // user data, so we add an extra Box layer:
            let state_dyn = Rc::clone(&state) as Rc<WindowState<dyn HandleEvent>>;
            let state_ptr = Box::into_raw(Box::new(state_dyn));
            winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, state_ptr as isize);

            state
        };

        Ok(WindowInner { state })
    }

    pub fn show(&self) {
        unsafe {
            winuser::ShowWindow(self.state.hwnd, winuser::SW_SHOWNORMAL);
        }
    }

    pub fn hide(&self) {
        unsafe {
            winuser::ShowWindow(self.state.hwnd, winuser::SW_HIDE);
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
            let hdc = winuser::GetDC(self.state.hwnd);
            if !hdc.is_null() {
                if let Some(rects) = rects {
                    let (layout, _) = Layout::new::<wingdi::RGNDATAHEADER>()
                        .extend(Layout::array::<windef::RECT>(rects.len()).unwrap())
                        .unwrap();
                    let ptr = alloc(layout) as *mut wingdi::RGNDATA;

                    let buffer_ptr = ptr::addr_of!((*ptr).Buffer) as *mut MaybeUninit<windef::RECT>;
                    let buffer = slice::from_raw_parts_mut(buffer_ptr, rects.len());
                    for (src, dst) in rects.iter().zip(buffer.iter_mut()) {
                        dst.write(windef::RECT {
                            left: src.x.round() as i32,
                            top: src.y.round() as i32,
                            right: (src.x + src.width).round() as i32,
                            bottom: (src.y + src.height).round() as i32,
                        });
                    }

                    let buffer =
                        slice::from_raw_parts(buffer_ptr as *const windef::RECT, rects.len());
                    let bounds = if buffer.is_empty() {
                        windef::RECT {
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

                    (*ptr).rdh = wingdi::RGNDATAHEADER {
                        dwSize: mem::size_of::<wingdi::RGNDATAHEADER>() as u32,
                        iType: wingdi::RDH_RECTANGLES,
                        nCount: rects.len() as u32,
                        nRgnSize: layout.size() as u32,
                        rcBound: bounds,
                    };

                    let rgn = wingdi::ExtCreateRegion(ptr::null(), layout.size() as u32, ptr);
                    wingdi::SelectClipRgn(hdc, rgn);
                    wingdi::DeleteObject(rgn as *mut c_void);

                    dealloc(ptr as *mut u8, layout);
                }

                let bitmap_info = wingdi::BITMAPINFO {
                    bmiHeader: wingdi::BITMAPINFOHEADER {
                        biSize: mem::size_of::<wingdi::BITMAPINFOHEADER>() as u32,
                        biWidth: bitmap.width() as i32,
                        biHeight: -(bitmap.height() as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: wingdi::BI_RGB,
                        ..mem::zeroed()
                    },
                    ..mem::zeroed()
                };

                wingdi::StretchDIBits(
                    hdc,
                    0,
                    0,
                    bitmap.width() as i32,
                    bitmap.height() as i32,
                    0,
                    0,
                    bitmap.width() as i32,
                    bitmap.height() as i32,
                    bitmap.data().as_ptr() as *const ntdef::VOID,
                    &bitmap_info,
                    wingdi::DIB_RGB_COLORS,
                    wingdi::SRCCOPY,
                );

                if rects.is_some() {
                    wingdi::SelectClipRgn(hdc, ptr::null_mut());
                }

                winuser::ReleaseDC(self.state.hwnd, hdc);
            }
        }
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        self.state.cursor.set(cursor);
        self.state.update_cursor();
    }

    pub fn set_mouse_position(&self, position: Point) {
        unsafe {
            let mut point = windef::POINT {
                x: position.x as c_int,
                y: position.y as c_int,
            };
            winuser::ClientToScreen(self.state.hwnd, &mut point);
            winuser::SetCursorPos(point.x, point.y);
        }
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.state.hwnd as *mut c_void,
            ..WindowsHandle::empty()
        })
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            winuser::DestroyWindow(self.state.hwnd);
        }
    }
}

pub unsafe extern "system" fn wnd_proc(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    let state_ptr = winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA)
        as *mut Rc<WindowState<dyn HandleEvent>>;
    if !state_ptr.is_null() {
        // Hold a reference to the WindowState for the duration of the wnd_proc, in case the
        // window is closed during an event handler
        let state = Rc::clone(&*state_ptr);

        match msg {
            winuser::WM_SETCURSOR => {
                if minwindef::LOWORD(lparam as minwindef::DWORD)
                    == winuser::HTCLIENT as minwindef::WORD
                {
                    state.update_cursor();
                    return 0;
                }
            }
            winuser::WM_ERASEBKGND => {
                return 1;
            }
            winuser::WM_PAINT => {
                let mut rects = Vec::new();

                let rgn = wingdi::CreateRectRgn(0, 0, 0, 0);
                winuser::GetUpdateRgn(hwnd, rgn, 0);
                let size = wingdi::GetRegionData(rgn, 0, ptr::null_mut());
                if size != 0 {
                    let align = mem::align_of::<wingdi::RGNDATA>();
                    let layout = Layout::from_size_align(size as usize, align).unwrap();
                    let ptr = alloc(layout) as *mut wingdi::RGNDATA;

                    let result = wingdi::GetRegionData(rgn, size, ptr);
                    if result == size {
                        let count = (*ptr).rdh.nCount as usize;

                        let buffer_ptr = ptr::addr_of!((*ptr).Buffer) as *const windef::RECT;
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
                wingdi::DeleteObject(rgn as *mut c_void);

                // Only validate the dirty region if we successfully invoked the event handler.
                // This ensures that if we receive an expose event during the App:new builder
                // callback, we will receive it again later.
                if state.handler.handle_event(Event::Expose(&rects)).is_some() {
                    winuser::ValidateRgn(hwnd, ptr::null_mut());
                }

                return 0;
            }
            winuser::WM_MOUSEMOVE => {
                let point = Point {
                    x: windowsx::GET_X_LPARAM(lparam) as f64,
                    y: windowsx::GET_Y_LPARAM(lparam) as f64,
                };
                state.handler.handle_event(Event::MouseMove(point));

                return 0;
            }
            winuser::WM_LBUTTONDOWN
            | winuser::WM_LBUTTONUP
            | winuser::WM_MBUTTONDOWN
            | winuser::WM_MBUTTONUP
            | winuser::WM_RBUTTONDOWN
            | winuser::WM_RBUTTONUP
            | winuser::WM_XBUTTONDOWN
            | winuser::WM_XBUTTONUP => {
                let button = match msg {
                    winuser::WM_LBUTTONDOWN | winuser::WM_LBUTTONUP => Some(MouseButton::Left),
                    winuser::WM_MBUTTONDOWN | winuser::WM_MBUTTONUP => Some(MouseButton::Middle),
                    winuser::WM_RBUTTONDOWN | winuser::WM_RBUTTONUP => Some(MouseButton::Right),
                    winuser::WM_XBUTTONDOWN | winuser::WM_XBUTTONUP => {
                        match winuser::GET_XBUTTON_WPARAM(wparam) {
                            winuser::XBUTTON1 => Some(MouseButton::Back),
                            winuser::XBUTTON2 => Some(MouseButton::Forward),
                            _ => None,
                        }
                    }
                    _ => None,
                };

                if let Some(button) = button {
                    let event = match msg {
                        winuser::WM_LBUTTONDOWN
                        | winuser::WM_MBUTTONDOWN
                        | winuser::WM_RBUTTONDOWN
                        | winuser::WM_XBUTTONDOWN => Some(Event::MouseDown(button)),
                        winuser::WM_LBUTTONUP
                        | winuser::WM_MBUTTONUP
                        | winuser::WM_RBUTTONUP
                        | winuser::WM_XBUTTONUP => Some(Event::MouseUp(button)),
                        _ => None,
                    };

                    if let Some(event) = event {
                        match event {
                            Event::MouseDown(_) => {
                                state.mouse_down_count.set(state.mouse_down_count.get() + 1);
                                if state.mouse_down_count.get() == 1 {
                                    winuser::SetCapture(hwnd);
                                }
                            }
                            Event::MouseUp(_) => {
                                state.mouse_down_count.set(state.mouse_down_count.get() - 1);
                                if state.mouse_down_count.get() == 0 {
                                    winuser::ReleaseCapture();
                                }
                            }
                            _ => {}
                        }

                        if state.handler.handle_event(event) == Some(Response::Capture) {
                            return 0;
                        }
                    }
                }
            }
            winuser::WM_MOUSEWHEEL | winuser::WM_MOUSEHWHEEL => {
                let delta = winuser::GET_WHEEL_DELTA_WPARAM(wparam) as f64 / 120.0;
                let point = match msg {
                    winuser::WM_MOUSEWHEEL => Point::new(0.0, delta),
                    winuser::WM_MOUSEHWHEEL => Point::new(delta, 0.0),
                    _ => unreachable!(),
                };

                if state.handler.handle_event(Event::Scroll(point)) == Some(Response::Capture) {
                    return 0;
                }
            }
            winuser::WM_CLOSE => {
                state.handler.handle_event(Event::Close);
                return 0;
            }
            winuser::WM_DESTROY => {
                drop(Box::from_raw(state_ptr));
                winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, 0);
            }
            _ => {}
        }
    }

    winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
}
