use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use super::app::{AppContextInner, AppState};
use crate::AppContext;

struct TimerState {
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

pub struct Timers {
    next_id: Cell<usize>,
    timers: RefCell<HashMap<usize, Rc<TimerState>>>,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            next_id: Cell::new(0),
            timers: RefCell::new(HashMap::new()),
        }
    }

    pub fn set_timer<T, H>(
        &self,
        app_state: &Rc<AppState>,
        duration: Duration,
        handler: H,
    ) -> TimerInner
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
            SetTimer(app_state.message_hwnd, timer_id, millis, None);
        }

        TimerInner {
            app_state: Rc::downgrade(app_state),
            timer_id,
        }
    }

    pub fn handle_timer(&self, app_state: &Rc<AppState>, timer_id: usize) {
        let timer_state = app_state.timers.timers.borrow().get(&timer_id).cloned();
        if let Some(timer_state) = timer_state {
            if let Ok(mut data) = app_state.data.try_borrow_mut() {
                if let Some(data) = &mut *data {
                    timer_state.handler.borrow_mut()(&mut **data, &app_state);
                }
            }
        }
    }

    pub fn kill_timers(&self, app_state: &AppState) {
        for (timer_id, _timer) in self.timers.take() {
            unsafe {
                let _ = KillTimer(app_state.message_hwnd, timer_id);
            }
        }
    }
}

#[derive(Clone)]
pub struct TimerInner {
    app_state: Weak<AppState>,
    timer_id: usize,
}

impl TimerInner {
    pub fn cancel(&self) {
        if let Some(app_state) = self.app_state.upgrade() {
            if let Some(_) = app_state.timers.timers.borrow_mut().remove(&self.timer_id) {
                unsafe {
                    let _ = KillTimer(app_state.message_hwnd, self.timer_id);
                }
            }
        }
    }
}
