use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;
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
    let state = unsafe { &*(info as *const TimerState) };

    let cx = AppContext::from_inner(AppContextInner::new(&state.app_state));
    state.handler.borrow_mut()(&cx);
}

struct TimerState {
    timer_ref: Cell<Option<CFRunLoopTimerRef>>,
    app_state: Rc<AppState>,
    handler: RefCell<Box<dyn FnMut(&AppContext)>>,
}

impl TimerState {
    fn cancel(&self) {
        if let Some(timer_ref) = self.timer_ref.take() {
            unsafe {
                CFRunLoopTimerInvalidate(timer_ref);
                CFRelease(timer_ref as CFTypeRef);
            }
        }
    }
}

pub struct Timers {
    timers: RefCell<HashMap<CFRunLoopTimerRef, Rc<TimerState>>>,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            timers: RefCell::new(HashMap::new()),
        }
    }

    pub fn set_timer<H>(
        &self,
        app_state: &Rc<AppState>,
        duration: Duration,
        handler: H,
    ) -> TimerInner
    where
        H: FnMut(&AppContext) + 'static,
    {
        let state = Rc::new(TimerState {
            timer_ref: Cell::new(None),
            app_state: Rc::clone(app_state),
            handler: RefCell::new(Box::new(handler)),
        });

        let mut context = CFRunLoopTimerContext {
            version: 0,
            info: Rc::as_ptr(&state) as *mut c_void,
            retain: Some(retain),
            release: Some(release),
            copyDescription: None,
        };

        let now = unsafe { CFAbsoluteTimeGetCurrent() };
        let interval = duration.as_secs_f64();

        let timer_ref = unsafe {
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
        state.timer_ref.set(Some(timer_ref));

        app_state.timers.timers.borrow_mut().insert(timer_ref, Rc::clone(&state));

        unsafe {
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddTimer(run_loop, timer_ref, kCFRunLoopCommonModes);
        }

        TimerInner { state }
    }

    pub fn shutdown(&self) {
        for timer in self.timers.take().into_values() {
            timer.cancel();
        }
    }
}

#[derive(Clone)]
pub struct TimerInner {
    state: Rc<TimerState>,
}

impl TimerInner {
    pub fn cancel(&self) {
        if let Some(timer_ref) = self.state.timer_ref.get() {
            self.state.app_state.timers.timers.borrow_mut().remove(&timer_ref);
        }

        self.state.cancel();
    }
}
