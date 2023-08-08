use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use super::app::{AppContextInner, AppState};
use crate::AppContext;

pub type TimerId = usize;

struct Timer {
    duration: Duration,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>)>>,
}

#[derive(Clone)]
struct QueueEntry {
    time: Instant,
    timer_id: TimerId,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.time.cmp(&other.time).reverse())
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time).reverse()
    }
}

pub struct TimerState {
    next_id: Cell<TimerId>,
    timers: RefCell<HashMap<usize, Rc<Timer>>>,
    queue: RefCell<BinaryHeap<QueueEntry>>,
}

impl TimerState {
    pub fn new() -> TimerState {
        TimerState {
            next_id: Cell::new(0),
            timers: RefCell::new(HashMap::new()),
            queue: RefCell::new(BinaryHeap::new()),
        }
    }

    pub fn next_time(&self) -> Option<Instant> {
        self.queue.borrow().peek().map(|e| e.time)
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
        let now = Instant::now();

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
            Rc::new(Timer {
                duration,
                handler: RefCell::new(Box::new(handler_wrapper)),
            }),
        );

        self.queue.borrow_mut().push(QueueEntry {
            time: now + duration,
            timer_id,
        });

        TimerHandleInner {
            app_state: Rc::downgrade(app_state),
            timer_id,
        }
    }

    pub fn poll(&self, data: &mut dyn Any, app_state: &Rc<AppState>) {
        let now = Instant::now();

        // Check with < and not <= so that we don't process a timer twice during this loop
        while self.next_time().map_or(false, |t| t < now) {
            let next = self.queue.borrow_mut().pop().unwrap();

            // If we don't find the timer in `self.timers`, it has been canceled
            if let Some(timer) = self.timers.borrow().get(&next.timer_id).cloned() {
                timer.handler.borrow_mut()(data, app_state);

                // If we fall behind by more than one timer interval, reset the timer's phase
                let next_time = (next.time + timer.duration).max(now);

                self.queue.borrow_mut().push(QueueEntry {
                    time: next_time,
                    timer_id: next.timer_id,
                })
            }
        }
    }
}

pub struct TimerHandleInner {
    app_state: Weak<AppState>,
    timer_id: TimerId,
}

impl TimerHandleInner {
    pub fn cancel(self) {
        let app_state = self
            .app_state
            .upgrade()
            .expect("attempted to use TimerHandle after App has been dropped");

        app_state.timer_state.timers.borrow_mut().remove(&self.timer_id);
    }
}
