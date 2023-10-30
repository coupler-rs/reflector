use std::cell::RefCell;
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
use crate::{AppMode, AppOptions, Result, TimerContext};

pub struct AppState {
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

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerInner
    where
        H: FnMut(&TimerContext) + 'static,
    {
        self.state.timers.set_timer(&self.state, duration, handler)
    }

    pub fn run(&self) -> Result<()> {
        autoreleasepool(|_| unsafe {
            NSApplication::sharedApplication().run();

            Ok(())
        })
    }

    pub fn exit(&self) {
        unsafe {
            NSApplication::sharedApplication().stop(None);
        }
    }

    pub fn poll(&self) -> Result<()> {
        Ok(())
    }

    pub fn shutdown(&self) {
        autoreleasepool(|_| {
            for window_state in self.state.windows.take().into_values() {
                window_state.close();
            }

            self.state.timers.shutdown();
        })
    }
}
