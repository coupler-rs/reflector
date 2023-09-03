use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;
use std::{mem, ptr, result};

use winapi::{shared::minwindef, shared::ntdef, um::errhandlingapi, um::winuser};

use super::window::wnd_proc;
use super::{hinstance, to_wstring, OsError, TimerHandleInner};
use crate::{App, AppContext, AppOptions, Error, IntoInnerError, Result};

pub struct AppState {
    pub class: minwindef::ATOM,
    pub data: RefCell<Option<Box<dyn Any>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            winuser::UnregisterClassW(self.class as *const ntdef::WCHAR, hinstance());
        }
    }
}

pub struct AppInner<T> {
    pub state: Rc<AppState>,
    _marker: PhantomData<T>,
}

impl<T: 'static> AppInner<T> {
    pub fn new<F>(_options: &AppOptions, build: F) -> Result<AppInner<T>>
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

        let cx = AppContext::from_inner(AppContextInner {
            state: &state,
            _marker: PhantomData,
        });
        let data = build(&cx)?;

        state.data.replace(Some(Box::new(data)));

        Ok(AppInner {
            state,
            _marker: PhantomData,
        })
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

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>> {
        if let Ok(mut data) = self.state.data.try_borrow_mut() {
            if let Some(data) = data.take() {
                return Ok(*data.downcast().unwrap());
            }
        }

        Err(IntoInnerError::new(
            Error::InsideEventHandler,
            App::from_inner(self),
        ))
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
    pub state: &'a Rc<AppState>,
    pub _marker: PhantomData<T>,
}

impl<'a, T> AppContextInner<'a, T> {
    pub(super) fn new(state: &'a Rc<AppState>) -> AppContextInner<'a, T> {
        AppContextInner {
            state,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, _duration: Duration, _handler: H) -> TimerHandleInner
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
