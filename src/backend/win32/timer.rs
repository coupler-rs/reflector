use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use super::app::{AppContextInner, AppState};
use crate::AppContext;

struct TimerState {
    timer_id: Cell<Option<usize>>,
    app_state: Rc<AppState>,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

impl TimerState {
    fn cancel(&self) {
        if let Some(timer_id) = self.timer_id.take() {
            let _ = unsafe { KillTimer(self.app_state.message_hwnd, timer_id) };
        }
    }
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

        let state = Rc::new(TimerState {
            timer_id: Cell::new(Some(timer_id)),
            app_state: Rc::clone(app_state),
            handler: RefCell::new(Box::new(handler_wrapper)),
        });

        self.timers.borrow_mut().insert(timer_id, Rc::clone(&state));

        unsafe {
            let millis = duration.as_millis() as u32;
            SetTimer(app_state.message_hwnd, timer_id, millis, None);
        }

        TimerInner { state }
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
        if let Some(timer_id) = self.state.timer_id.get() {
            self.state.app_state.timers.timers.borrow_mut().remove(&timer_id);
        }

        self.state.cancel();
    }
}
