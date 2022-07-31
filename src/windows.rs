use crate::{
    App, AppContext, CloseError, Cursor, Error, Event, MouseButton, Point, Rect, Response, Result,
    Window, WindowOptions,
};

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::rc::Rc;
use std::{fmt, mem, ptr, result};

use raw_window_handle::{windows::WindowsHandle, RawWindowHandle};
use winapi::{
    shared::minwindef, shared::ntdef, shared::windef, shared::windowsx, um::errhandlingapi,
    um::wingdi, um::winnt, um::winuser,
};

extern "C" {
    static __ImageBase: winnt::IMAGE_DOS_HEADER;
}

fn to_wstring<S: AsRef<OsStr> + ?Sized>(str: &S) -> Vec<ntdef::WCHAR> {
    let mut wstr: Vec<ntdef::WCHAR> = str.as_ref().encode_wide().collect();
    wstr.push(0);
    wstr
}

#[derive(Debug)]
pub struct OsError {
    code: minwindef::DWORD,
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.code)
    }
}

struct AppState<T> {
    class: minwindef::ATOM,
    running: Cell<bool>,
    data: RefCell<Option<T>>,
}

impl<T> Drop for AppState<T> {
    fn drop(&mut self) {
        unsafe {
            winuser::UnregisterClassW(
                self.class as *const ntdef::WCHAR,
                &__ImageBase as *const winnt::IMAGE_DOS_HEADER as minwindef::HINSTANCE,
            );
        }
    }
}

pub struct AppInner<T> {
    state: Rc<AppState<T>>,
}

impl<T> AppInner<T> {
    pub fn new<F>(build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        let class = unsafe {
            let class_name = to_wstring(&format!("window-{}", uuid::Uuid::new_v4().to_simple()));

            let wnd_class = winuser::WNDCLASSW {
                style: winuser::CS_HREDRAW | winuser::CS_VREDRAW | winuser::CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: &__ImageBase as *const winnt::IMAGE_DOS_HEADER as minwindef::HINSTANCE,
                hIcon: ptr::null_mut(),
                hCursor: winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_ARROW),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            let class = winuser::RegisterClassW(&wnd_class);
            if class == 0 {
                return Err(Error::Os(OsError {
                    code: errhandlingapi::GetLastError(),
                }));
            }

            class
        };

        let state = Rc::new(AppState {
            class,
            running: Cell::new(false),
            data: RefCell::new(None),
        });

        let cx = AppContext::from_inner(AppContextInner { state: &state });
        let data = build(&cx)?;

        state.data.replace(Some(data));

        Ok(AppInner { state })
    }

    pub fn run(&self) -> Result<()> {
        if self.state.running.get() || self.state.data.try_borrow().is_err() {
            return Err(Error::InsideEventHandler);
        }

        self.state.running.set(true);
        while self.state.running.get() {
            unsafe {
                let mut msg: winuser::MSG = mem::zeroed();

                let result = winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
                if result < 0 {
                    return Err(Error::Os(OsError {
                        code: errhandlingapi::GetLastError(),
                    }));
                } else if result == 0 {
                    // ignore WM_QUIT messages
                    continue;
                }

                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            }
        }

        Ok(())
    }

    pub fn poll(&self) -> Result<()> {
        Ok(())
    }

    pub fn into_inner(self) -> result::Result<T, CloseError<App<T>>> {
        unimplemented!()
    }
}

impl<T> Drop for AppInner<T> {
    fn drop(&mut self) {
        if let Ok(mut data) = self.state.data.try_borrow_mut() {
            drop(data.take());
        }
    }
}

pub struct AppContextInner<'a, T> {
    state: &'a Rc<AppState<T>>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub fn exit(&self) {
        self.state.running.set(false);
    }
}

trait HandleEvent {
    fn handle_event(&self, event: Event) -> Option<Response>;
}

struct Handler<T, H> {
    app_state: Rc<AppState<T>>,
    handler: RefCell<H>,
}

impl<T, H> HandleEvent for Handler<T, H>
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

struct WindowState {
    hdc: Cell<Option<windef::HDC>>,
    mouse_down_count: Cell<isize>,
    cursor: Cell<Cursor>,
    handler: Box<dyn HandleEvent>,
}

impl WindowState {
    unsafe fn from_hwnd(hwnd: windef::HWND) -> *mut WindowState {
        winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA) as *mut WindowState
    }

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

const TIMER_ID: usize = 1;
const TIMER_INTERVAL: u32 = 16;

pub struct WindowInner {
    hwnd: windef::HWND,
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
        let hwnd = unsafe {
            let flags = winuser::WS_CLIPCHILDREN
                | winuser::WS_CLIPSIBLINGS
                | winuser::WS_CAPTION
                | winuser::WS_SIZEBOX
                | winuser::WS_SYSMENU
                | winuser::WS_MINIMIZEBOX
                | winuser::WS_MAXIMIZEBOX;

            let mut rect = windef::RECT {
                left: options.rect.x.round() as i32,
                top: options.rect.y.round() as i32,
                right: (options.rect.x + options.rect.width).round() as i32,
                bottom: (options.rect.y + options.rect.height).round() as i32,
            };
            winuser::AdjustWindowRectEx(&mut rect, flags, minwindef::FALSE, 0);

            let window_name = to_wstring(&options.title);

            let hwnd = winuser::CreateWindowExW(
                0,
                cx.inner.state.class as *const ntdef::WCHAR,
                window_name.as_ptr(),
                flags,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Err(Error::Os(OsError {
                    code: errhandlingapi::GetLastError(),
                }));
            }

            let state = Rc::into_raw(Rc::new(WindowState {
                hdc: Cell::new(None),
                mouse_down_count: Cell::new(0),
                cursor: Cell::new(Cursor::Arrow),
                handler: Box::new(Handler {
                    app_state: Rc::clone(cx.inner.state),
                    handler: RefCell::new(handler),
                }),
            }));

            winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, state as isize);

            winuser::SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL, None);

            hwnd
        };

        Ok(WindowInner { hwnd })
    }

    pub fn show(&self) {
        unsafe {
            winuser::ShowWindow(self.hwnd, winuser::SW_SHOWNORMAL);
        }
    }

    pub fn hide(&self) {
        unsafe {
            winuser::ShowWindow(self.hwnd, winuser::SW_HIDE);
        }
    }

    pub fn request_display(&self) {
        unsafe {
            winuser::InvalidateRect(self.hwnd, ptr::null(), minwindef::FALSE);
        }
    }

    pub fn request_display_rect(&self, rect: Rect) {
        unsafe {
            let rect = windef::RECT {
                left: rect.x.round() as winnt::LONG,
                top: rect.y.round() as winnt::LONG,
                right: (rect.x + rect.width).round() as winnt::LONG,
                bottom: (rect.y + rect.height).round() as winnt::LONG,
            };

            winuser::InvalidateRect(self.hwnd, &rect, minwindef::FALSE);
        }
    }

    pub fn update_contents(&self, buffer: &[u32], width: usize, height: usize) {
        assert!(
            width * height == buffer.len(),
            "invalid framebuffer dimensions"
        );

        let state = unsafe { &*WindowState::from_hwnd(self.hwnd) };

        unsafe {
            let hdc = if let Some(hdc) = state.hdc.get() {
                hdc
            } else {
                winuser::GetDC(self.hwnd)
            };

            if !hdc.is_null() {
                let bitmap_info = wingdi::BITMAPINFO {
                    bmiHeader: wingdi::BITMAPINFOHEADER {
                        biSize: mem::size_of::<wingdi::BITMAPINFOHEADER>() as u32,
                        biWidth: width as i32,
                        biHeight: -(height as i32),
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
                    width as i32,
                    height as i32,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    buffer.as_ptr() as *const ntdef::VOID,
                    &bitmap_info,
                    wingdi::DIB_RGB_COLORS,
                    wingdi::SRCCOPY,
                );

                if state.hdc.get().is_none() {
                    winuser::ReleaseDC(self.hwnd, hdc);
                }
            }
        }
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        let state = unsafe { &*WindowState::from_hwnd(self.hwnd) };

        state.cursor.set(cursor);
        state.update_cursor();
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.hwnd as *mut std::ffi::c_void,
            ..WindowsHandle::empty()
        })
    }

    pub fn close(self) -> result::Result<(), CloseError<Window>> {
        unimplemented!()
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            winuser::KillTimer(self.hwnd, TIMER_ID);

            let state_ptr = WindowState::from_hwnd(self.hwnd);
            winuser::DestroyWindow(self.hwnd);
            drop(Rc::from_raw(state_ptr));
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    let state_ptr = WindowState::from_hwnd(hwnd);
    if !state_ptr.is_null() {
        let state_rc = Rc::from_raw(state_ptr);
        let state = Rc::clone(&state_rc);
        let _ = Rc::into_raw(state_rc);

        match msg {
            winuser::WM_TIMER => {
                if wparam == TIMER_ID {
                    state.handler.handle_event(Event::Frame);
                }
                return 0;
            }
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
                let mut paint_struct: winuser::PAINTSTRUCT = mem::zeroed();
                let hdc = winuser::BeginPaint(hwnd, &mut paint_struct);
                if !hdc.is_null() {
                    state.hdc.set(Some(hdc));
                }

                state.handler.handle_event(Event::Display);

                state.hdc.set(None);
                winuser::EndPaint(hwnd, &paint_struct);

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
                state.handler.handle_event(Event::RequestClose);
                return 0;
            }
            _ => {}
        }
    }

    winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
}
