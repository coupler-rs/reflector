use crate::{App, AppContext, Error, IntoInnerError, Result};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::{mem, ptr, result};

use winapi::{shared::minwindef, shared::ntdef, um::errhandlingapi, um::winuser};

use super::window::wnd_proc;
use super::{hinstance, to_wstring, OsError, TimerHandleInner};

pub struct AppState<T> {
    pub class: minwindef::ATOM,
    pub data: RefCell<Option<T>>,
}

impl<T> Drop for AppState<T> {
    fn drop(&mut self) {
        unsafe {
            winuser::UnregisterClassW(self.class as *const ntdef::WCHAR, hinstance());
        }
    }
}

pub struct AppInner<T> {
    pub state: Rc<AppState<T>>,
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
                hInstance: hinstance(),
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

    pub fn run(&mut self) -> Result<()> {
        if self.state.data.try_borrow().is_err() {
            return Err(Error::InsideEventHandler);
        }

        loop {
            unsafe {
                let mut msg: winuser::MSG = mem::zeroed();

                let result = winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
                if result < 0 {
                    return Err(Error::Os(OsError {
                        code: errhandlingapi::GetLastError(),
                    }));
                } else if result == 0 {
                    return Ok(());
                }

                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            }
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        if self.state.data.try_borrow().is_err() {
            return Err(Error::InsideEventHandler);
        }

        loop {
            unsafe {
                let mut msg: winuser::MSG = mem::zeroed();

                let result =
                    winuser::PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, winuser::PM_REMOVE);
                if result == 0 {
                    return Ok(());
                }

                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            }
        }
    }

    fn take_data(&self) -> Option<T> {
        if let Ok(mut data) = self.state.data.try_borrow_mut() {
            return data.take();
        }

        None
    }

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>>
    where
        T: 'static,
    {
        if let Some(data) = self.take_data() {
            Ok(data)
        } else {
            Err(IntoInnerError::new(
                Error::InsideEventHandler,
                App::from_inner(self),
            ))
        }
    }
}

impl<T> Drop for AppInner<T> {
    fn drop(&mut self) {
        drop(self.take_data());
    }
}

pub struct AppContextInner<'a, T> {
    pub state: &'a Rc<AppState<T>>,
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
        unsafe {
            winuser::PostQuitMessage(0);
        }
    }
}
