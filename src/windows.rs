use crate::{AppContext, Error, Event, Response, Result, WindowOptions};

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::rc::Rc;
use std::{fmt, mem, ptr};

use winapi::{
    shared::minwindef, shared::ntdef, shared::windef, um::errhandlingapi, um::winnt, um::winuser,
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
                lpfnWndProc: Some(wnd_proc::<T>),
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

struct WindowState<T> {
    app_state: Rc<AppState<T>>,
    handler: RefCell<Box<dyn FnMut(&mut T, &AppContext<T>, Event) -> Response>>,
}

impl<T> WindowState<T> {
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

            let state = Box::into_raw(Box::new(WindowState {
                app_state: Rc::clone(cx.inner.state),
                handler: RefCell::new(Box::new(handler)),
            }));

            winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, state as isize);

            winuser::ShowWindow(hwnd, winuser::SW_SHOWNORMAL);
            winuser::UpdateWindow(hwnd);

            hwnd
        };

        Ok(WindowInner { hwnd })
    }
}

unsafe extern "system" fn wnd_proc<T>(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    let state_ptr = winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA) as *mut WindowState<T>;
    if !state_ptr.is_null() {
        let state = &*state_ptr;

        match msg {
            winuser::WM_CLOSE => {
                state.handle_event(Event::RequestClose);
                return 0;
            }
            _ => {}
        }
    }

    winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
}
