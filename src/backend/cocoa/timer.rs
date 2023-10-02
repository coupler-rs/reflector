use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr;
use std::rc::{Rc, Weak};
use std::time::Duration;

use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation::date::CFAbsoluteTimeGetCurrent;
use core_foundation::runloop::*;

use super::app::{AppContextInner, AppState};
use crate::AppContext;

extern "C" fn retain(info: *const c_void) -> *const c_void {
    unsafe {
        Rc::increment_strong_count(info as *const TimerState);
    }

    info
}

extern "C" fn release(info: *const c_void) {
    unsafe {
        Rc::decrement_strong_count(info as *const TimerState);
    }
}

extern "C" fn callback(_timer: CFRunLoopTimerRef, info: *mut c_void) {
    let timer_state = unsafe { &*(info as *const TimerState) };

    if let Some(app_state) = timer_state.app_state.upgrade() {
        if let Ok(mut data) = app_state.data.try_borrow_mut() {
            if let Some(data) = &mut *data {
                timer_state.handler.borrow_mut()(&mut **data, &app_state);
            }
        }
    }
}

struct TimerState {
    app_state: Weak<AppState>,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

pub struct Timers {
    timers: RefCell<HashSet<CFRunLoopTimerRef>>,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            timers: RefCell::new(HashSet::new()),
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
        let mut handler = handler;
        let handler_wrapper = move |data_any: &mut dyn Any, app_state: &Rc<AppState>| {
            let data = data_any.downcast_mut::<T>().unwrap();
            let cx = AppContext::from_inner(AppContextInner::new(app_state));
            handler(data, &cx)
        };

        let timer_state = Rc::new(TimerState {
            app_state: Rc::downgrade(app_state),
            handler: RefCell::new(Box::new(handler_wrapper)),
        });

        let mut context = CFRunLoopTimerContext {
            version: 0,
            info: Rc::as_ptr(&timer_state) as *mut c_void,
            retain: Some(retain),
            release: Some(release),
            copyDescription: None,
        };

        let timer = unsafe {
            let now = CFAbsoluteTimeGetCurrent();
            let interval = duration.as_secs_f64();

            CFRunLoopTimerCreate(
                ptr::null(),
                now + interval,
                interval,
                0,
                0,
                callback,
                &mut context,
            )
        };

        app_state.timers.timers.borrow_mut().insert(timer);

        unsafe {
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddTimer(run_loop, timer, kCFRunLoopCommonModes);
        }

        TimerHandleInner {
            app_state: Rc::downgrade(app_state),
            timer,
        }
    }
}

impl Drop for Timers {
    fn drop(&mut self) {
        for timer in self.timers.take() {
            unsafe {
                CFRunLoopTimerInvalidate(timer);
                CFRelease(timer as CFTypeRef);
            }
        }
    }
}

pub struct TimerHandleInner {
    app_state: Weak<AppState>,
    timer: CFRunLoopTimerRef,
}

impl TimerHandleInner {
    pub fn cancel(self) {
        if let Some(app_state) = self.app_state.upgrade() {
            if app_state.timers.timers.borrow_mut().remove(&self.timer) {
                unsafe {
                    CFRunLoopTimerInvalidate(self.timer);
                    CFRelease(self.timer as CFTypeRef);
                }
            }
        }
    }
}
