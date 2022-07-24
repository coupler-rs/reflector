use crate::{AppContext, Error, Event, Response, Result, WindowOptions};

use std::cell::RefCell;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::rc::Rc;
use std::{fmt, ptr};

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
            data: RefCell::new(None),
        });

        let cx = AppContext::from_inner(AppContextInner { state: &state });
        let data = build(&cx)?;

        state.data.replace(Some(data));

        Ok(AppInner { state })
    }

    pub fn run(&self) {}

    pub fn poll(&self) {}
}

impl<T> Drop for AppInner<T> {
    fn drop(&mut self) {
        drop(self.state.data.take());
    }
}

pub struct AppContextInner<'a, T> {
    state: &'a Rc<AppState<T>>,
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
}

unsafe extern "system" fn wnd_proc(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
}
