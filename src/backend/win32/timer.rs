use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;
use std::rc::{Rc, Weak};
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::UI::WindowsAndMessaging::{
    self as msg, CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, KillTimer,
    RegisterClassW, SetTimer, SetWindowLongPtrW, UnregisterClassW, HCURSOR, HICON, HMENU,
    WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WNDCLASS_STYLES,
};

use super::app::{AppContextInner, AppState};
use super::{class_name, hinstance, to_wstring};
use crate::AppContext;
use crate::Result;

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
                    let timer_state = app_state.timers.timers.borrow().get(&wparam.0).cloned();
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
    class: PCWSTR,
    hwnd: HWND,
    next_id: Cell<usize>,
    timers: RefCell<HashMap<usize, Rc<TimerState>>>,
}

impl Timers {
    pub fn new() -> Result<Timers> {
        let class_name = to_wstring(&class_name("timers-"));

        let wnd_class = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance(),
            hIcon: HICON(0),
            hCursor: HCURSOR(0),
            hbrBackground: HBRUSH(0),
            lpszMenuName: PCWSTR(ptr::null()),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };

        let class = unsafe { RegisterClassW(&wnd_class) };
        if class == 0 {
            return Err(windows::core::Error::from_win32().into());
        }
        let class = PCWSTR(class as *const u16);

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                PCWSTR(ptr::null()),
                WINDOW_STYLE(0),
                msg::CW_USEDEFAULT,
                msg::CW_USEDEFAULT,
                0,
                0,
                HWND(0),
                HMENU(0),
                hinstance(),
                None,
            )
        };
        if hwnd == HWND(0) {
            return Err(windows::core::Error::from_win32().into());
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
                let _ = KillTimer(self.hwnd, timer_id);
            }
        }

        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(self.class, hinstance());
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
                    let _ = KillTimer(app_state.timers.hwnd, self.timer_id);
                }
            }
        }
    }
}
