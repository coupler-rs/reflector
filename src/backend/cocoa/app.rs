use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use objc2::rc::{autoreleasepool, Id};
use objc2::runtime::AnyClass;
use objc2::ClassType;

use icrate::AppKit::{NSApplication, NSApplicationActivationPolicyRegular, NSCursor, NSImage};
use icrate::Foundation::{NSPoint, NSSize, NSThread};

use super::display_links::DisplayLinks;
use super::timer::{TimerInner, Timers};
use super::window::{View, WindowState};
use crate::{AppMode, AppOptions, Error, Result, TimerContext};

struct RunGuard<'a> {
    running: &'a Cell<bool>,
}

impl<'a> RunGuard<'a> {
    fn new(running: &'a Cell<bool>) -> Result<RunGuard<'a>> {
        if running.get() {
            return Err(Error::AlreadyRunning);
        }

        running.set(true);

        Ok(RunGuard { running })
    }
}

impl<'a> Drop for RunGuard<'a> {
    fn drop(&mut self) {
        self.running.set(false);
    }
}

pub struct AppState {
    pub open: Cell<bool>,
    pub running: Cell<bool>,
    pub class: &'static AnyClass,
    pub empty_cursor: Id<NSCursor>,
    pub timers: Timers,
    pub display_links: DisplayLinks,
    pub windows: RefCell<HashMap<*const WindowState, Rc<WindowState>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            View::unregister_class(self.class);
        }
    }
}

pub struct AppInner {
    pub state: Rc<AppState>,
}

impl AppInner {
    pub fn new(options: &AppOptions) -> Result<AppInner> {
        autoreleasepool(|_| {
            assert!(
                NSThread::isMainThread_class(),
                "App must be created on the main thread"
            );

            let class = View::register_class()?;

            let empty_cursor = unsafe {
                let empty_cursor_image =
                    NSImage::initWithSize(NSImage::alloc(), NSSize::new(1.0, 1.0));
                let empty_cursor = NSCursor::initWithImage_hotSpot(
                    NSCursor::alloc(),
                    &empty_cursor_image,
                    NSPoint::new(0.0, 0.0),
                );

                empty_cursor
            };

            let state = Rc::new(AppState {
                open: Cell::new(true),
                running: Cell::new(false),
                class,
                empty_cursor,
                timers: Timers::new(),
                display_links: DisplayLinks::new(),
                windows: RefCell::new(HashMap::new()),
            });

            state.display_links.init(&state);

            if options.mode == AppMode::Owner {
                unsafe {
                    let app = NSApplication::sharedApplication();
                    app.setActivationPolicy(NSApplicationActivationPolicyRegular);
                    app.activateIgnoringOtherApps(true);
                }
            }

            Ok(AppInner { state })
        })
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> Result<TimerInner>
    where
        H: FnMut(&TimerContext) + 'static,
    {
        if !self.state.open.get() {
            return Err(Error::AppDropped);
        }

        Ok(self.state.timers.set_timer(&self.state, duration, handler))
    }

    pub fn run(&self) -> Result<()> {
        autoreleasepool(|_| unsafe {
            if !self.state.open.get() {
                return Err(Error::AppDropped);
            }

            let _run_guard = RunGuard::new(&self.state.running)?;

            NSApplication::sharedApplication().run();

            Ok(())
        })
    }

    pub fn exit(&self) {
        autoreleasepool(|_| unsafe {
            NSApplication::sharedApplication().stop(None);
        })
    }

    pub fn poll(&self) -> Result<()> {
        if !self.state.open.get() {
            return Err(Error::AppDropped);
        }

        let _run_guard = RunGuard::new(&self.state.running)?;

        Ok(())
    }

    pub fn shutdown(&self) {
        autoreleasepool(|_| {
            self.state.open.set(false);

            for window_state in self.state.windows.take().into_values() {
                window_state.close();
            }

            self.state.timers.shutdown();
            self.state.display_links.shutdown();
        })
    }
}
