use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;
use std::rc::{Rc, Weak};
use std::time::Duration;

use winapi::{
    shared::basetsd, shared::minwindef, shared::ntdef, shared::windef, um::errhandlingapi,
    um::winuser,
};

use super::app::{AppContextInner, AppState};
use super::{hinstance, to_wstring, OsError};
use crate::AppContext;
use crate::{Error, Result};

pub unsafe extern "system" fn wnd_proc(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    let app_state_ptr = winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA) as *mut AppState;
    if !app_state_ptr.is_null() {
        let app_state_weak = Weak::from_raw(app_state_ptr);
        let app_state = app_state_weak.clone();
        let _ = app_state_weak.into_raw();

        match msg {
            winuser::WM_TIMER => {
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
            winuser::WM_DESTROY => {
                drop(Weak::from_raw(app_state_ptr));
                winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, 0);
            }
            _ => {}
        }
    }

    winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
}

struct TimerState {
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

pub struct Timers {
    class: minwindef::ATOM,
    hwnd: windef::HWND,
    next_id: Cell<basetsd::UINT_PTR>,
    timers: RefCell<HashMap<basetsd::UINT_PTR, Rc<TimerState>>>,
}

impl Timers {
    pub fn new() -> Result<Timers> {
        let class_name = to_wstring(&format!("timers-{}", uuid::Uuid::new_v4().to_simple()));

        let wnd_class = winuser::WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance(),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        let class = unsafe { winuser::RegisterClassW(&wnd_class) };
        if class == 0 {
            return Err(Error::Os(OsError {
                code: unsafe { errhandlingapi::GetLastError() },
            }));
        }

        let hwnd = unsafe {
            winuser::CreateWindowExW(
                0,
                class as *const ntdef::WCHAR,
                ptr::null_mut(),
                0,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                hinstance(),
                ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(Error::Os(OsError {
                code: unsafe { errhandlingapi::GetLastError() },
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
            winuser::SetWindowLongPtrW(self.hwnd, winuser::GWLP_USERDATA, state_ptr as isize);
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
            let millis = duration.as_millis() as minwindef::UINT;
            winuser::SetTimer(self.hwnd, timer_id, millis, None);
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
                winuser::KillTimer(self.hwnd, timer_id);
            }
        }

        unsafe {
            winuser::DestroyWindow(self.hwnd);
            winuser::UnregisterClassW(self.class as *const ntdef::WCHAR, hinstance());
        }
    }
}

pub struct TimerHandleInner {
    app_state: Weak<AppState>,
    timer_id: basetsd::UINT_PTR,
}

impl TimerHandleInner {
    pub fn cancel(self) {
        if let Some(app_state) = self.app_state.upgrade() {
            if let Some(_) = app_state.timers.timers.borrow_mut().remove(&self.timer_id) {
                unsafe {
                    winuser::KillTimer(app_state.timers.hwnd, self.timer_id);
                }
            }
        }
    }
}
