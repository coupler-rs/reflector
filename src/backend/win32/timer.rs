use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;
use std::rc::{Rc, Weak};
use std::time::Duration;

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    self as msg, CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, KillTimer,
    RegisterClassW, SetTimer, SetWindowLongPtrW, UnregisterClassW, WNDCLASSW,
};

use super::app::{AppContextInner, AppState};
use super::{class_name, hinstance, to_wstring, OsError};
use crate::AppContext;
use crate::{Error, Result};

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let app_state_ptr = GetWindowLongPtrW(hwnd, msg::GWLP_USERDATA) as *mut AppState;
    if !app_state_ptr.is_null() {
        let app_state_weak = Weak::from_raw(app_state_ptr);
        let app_state = app_state_weak.clone();
        let _ = app_state_weak.into_raw();

        match msg {
            msg::WM_TIMER => {
                if let Some(app_state) = app_state.upgrade() {
                    let timer_state = app_state.timers.timers.borrow().get(&wparam).cloned();
                    if let Some(timer_state) = timer_state {
                        if let Ok(mut data) = app_state.data.try_borrow_mut() {
                            if let Some(data) = &mut *data {
                                timer_state.handler.borrow_mut()(&mut **data, &app_state);
                            }
                        }
                    }
                }
            }
            msg::WM_DESTROY => {
                drop(Weak::from_raw(app_state_ptr));
                SetWindowLongPtrW(hwnd, msg::GWLP_USERDATA, 0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

struct TimerState {
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

pub struct Timers {
    class: u16,
    hwnd: HWND,
    next_id: Cell<usize>,
    timers: RefCell<HashMap<usize, Rc<TimerState>>>,
}

impl Timers {
    pub fn new() -> Result<Timers> {
        let class_name = to_wstring(&class_name("timers-"));

        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance(),
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        let class = unsafe { RegisterClassW(&wnd_class) };
        if class == 0 {
            return Err(Error::Os(OsError {
                code: unsafe { GetLastError() },
            }));
        }

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class as PCWSTR,
                ptr::null_mut(),
                0,
                msg::CW_USEDEFAULT,
                msg::CW_USEDEFAULT,
                0,
                0,
                0,
                0,
                hinstance(),
                ptr::null_mut(),
            )
        };
        if hwnd == 0 {
            return Err(Error::Os(OsError {
                code: unsafe { GetLastError() },
            }));
        }

        Ok(Timers {
            class,
            hwnd,
            next_id: Cell::new(0),
            timers: RefCell::new(HashMap::new()),
        })
    }

    // Should only be called once during setup. Calling this multiple times will result in leaks
    pub fn set_app_state(&self, app_state: &Rc<AppState>) {
        let state_ptr = Weak::into_raw(Rc::downgrade(app_state));
        unsafe {
            SetWindowLongPtrW(self.hwnd, msg::GWLP_USERDATA, state_ptr as isize);
        }
    }

    pub fn set_timer<T, H>(
        &self,
        app_state: &Rc<AppState>,
        duration: Duration,
        handler: H,
    ) -> TimerHandleInner
    where
        T: 'static,
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        let timer_id = self.next_id.get();
        self.next_id.set(timer_id + 1);

        let mut handler = handler;
        let handler_wrapper = move |data_any: &mut dyn Any, app_state: &Rc<AppState>| {
            let data = data_any.downcast_mut::<T>().unwrap();
            let cx = AppContext::from_inner(AppContextInner::new(app_state));
            handler(data, &cx)
        };

        self.timers.borrow_mut().insert(
            timer_id,
            Rc::new(TimerState {
                handler: RefCell::new(Box::new(handler_wrapper)),
            }),
        );

        unsafe {
            let millis = duration.as_millis() as u32;
            SetTimer(self.hwnd, timer_id, millis, None);
        }

        TimerHandleInner {
            app_state: Rc::downgrade(app_state),
            timer_id,
        }
    }
}

impl Drop for Timers {
    fn drop(&mut self) {
        for (timer_id, _timer) in self.timers.take() {
            unsafe {
                KillTimer(self.hwnd, timer_id);
            }
        }

        unsafe {
            DestroyWindow(self.hwnd);
            UnregisterClassW(self.class as PCWSTR, hinstance());
        }
    }
}

pub struct TimerHandleInner {
    app_state: Weak<AppState>,
    timer_id: usize,
}

impl TimerHandleInner {
    pub fn cancel(self) {
        if let Some(app_state) = self.app_state.upgrade() {
            if let Some(_) = app_state.timers.timers.borrow_mut().remove(&self.timer_id) {
                unsafe {
                    KillTimer(app_state.timers.hwnd, self.timer_id);
                }
            }
        }
    }
}
