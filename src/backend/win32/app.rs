use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;
use std::{mem, result};

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    self as msg, DispatchMessageW, GetMessageW, PeekMessageW, PostQuitMessage, TranslateMessage,
    MSG,
};

use super::dpi::DpiFns;
use super::timer::{TimerHandleInner, Timers};
use super::window;
use crate::{App, AppContext, AppMode, AppOptions, Error, IntoInnerError, Result};

pub struct AppState {
    pub class: PCWSTR,
    pub dpi: DpiFns,
    pub timers: Timers,
    pub data: RefCell<Option<Box<dyn Any>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            window::unregister_class(self.class);
        }
    }
}

pub struct AppInner<T> {
    pub state: Rc<AppState>,
    _marker: PhantomData<T>,
}

impl<T: 'static> AppInner<T> {
    pub fn new<F>(options: &AppOptions, build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        let class = window::register_class()?;

        let dpi = DpiFns::load();
        if options.mode == AppMode::Owner {
            dpi.set_dpi_aware();
        }

        let timers = Timers::new()?;

        let state = Rc::new(AppState {
            class,
            dpi,
            timers,
            data: RefCell::new(None),
        });

        state.timers.set_app_state(&state);

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
                let mut msg: MSG = mem::zeroed();

                let result = GetMessageW(&mut msg, HWND(0), 0, 0);
                if result.0 < 0 {
                    return Err(windows::core::Error::from_win32().into());
                } else if result.0 == 0 {
                    return Ok(());
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        if self.state.data.try_borrow().is_err() {
            return Err(Error::InsideEventHandler);
        }

        loop {
            unsafe {
                let mut msg: MSG = mem::zeroed();

                let result = PeekMessageW(&mut msg, HWND(0), 0, 0, msg::PM_REMOVE);
                if result.0 == 0 {
                    return Ok(());
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
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

impl<'a, T: 'static> AppContextInner<'a, T> {
    pub(super) fn new(state: &'a Rc<AppState>) -> AppContextInner<'a, T> {
        AppContextInner {
            state,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerHandleInner
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        self.state.timers.set_timer(self.state, duration, handler)
    }

    pub fn exit(&self) {
        unsafe {
            PostQuitMessage(0);
        }
    }
}
