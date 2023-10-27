use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::result;
use std::time::Duration;

use objc2::rc::{autoreleasepool, Id};
use objc2::runtime::AnyClass;
use objc2::ClassType;

use icrate::AppKit::{NSApplication, NSApplicationActivationPolicyRegular, NSCursor, NSImage};
use icrate::Foundation::{NSPoint, NSSize, NSThread};

use super::display_links::DisplayLinks;
use super::timer::{TimerInner, Timers};
use super::window::View;
use crate::{App, AppContext, AppMode, AppOptions, Error, IntoInnerError, Result};

pub struct AppState {
    pub class: &'static AnyClass,
    pub empty_cursor: Id<NSCursor>,
    pub timers: Timers,
    pub display_links: DisplayLinks,
    pub windows: RefCell<HashMap<*const View, Id<View>>>,
    pub data: RefCell<Option<Box<dyn Any>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            View::unregister_class(self.class);
        }
    }
}

pub struct AppInner<T> {
    pub state: Rc<AppState>,
    _marker: PhantomData<T>,
}

impl<T: 'static> AppInner<T> {
    pub fn new<F>(options: &AppOptions, build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
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
                data: RefCell::new(None),
            });

            state.display_links.init(&state);

            let cx = AppContext::from_inner(AppContextInner {
                state: &state,
                _marker: PhantomData,
            });
            let data = build(&cx)?;

            state.data.replace(Some(Box::new(data)));

            if options.mode == AppMode::Owner {
                unsafe {
                    let app = NSApplication::sharedApplication();
                    app.setActivationPolicy(NSApplicationActivationPolicyRegular);
                    app.activateIgnoringOtherApps(true);
                }
            }

            Ok(AppInner {
                state,
                _marker: PhantomData,
            })
        })
    }

    pub fn run(&mut self) -> Result<()> {
        autoreleasepool(|_| unsafe {
            NSApplication::sharedApplication().run();

            Ok(())
        })
    }

    pub fn poll(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn into_inner(self) -> result::Result<T, IntoInnerError<App<T>>> {
        if let Ok(mut data) = self.state.data.try_borrow_mut() {
            if let Some(data) = data.take() {
                return Ok(*data.downcast().unwrap());
            }
        }

        Err(IntoInnerError::new(
            Error::InsideEventHandler,
            App::from_inner(self),
        ))
    }
}

impl<T> Drop for AppInner<T> {
    fn drop(&mut self) {
        autoreleasepool(|_| {
            if let Ok(mut data) = self.state.data.try_borrow_mut() {
                drop(data.take());
            }
        })
    }
}

pub struct AppContextInner<'a, T> {
    pub state: &'a Rc<AppState>,
    _marker: PhantomData<T>,
}

impl<'a, T: 'static> AppContextInner<'a, T> {
    pub(super) fn new(state: &'a Rc<AppState>) -> AppContextInner<'a, T> {
        AppContextInner {
            state,
            _marker: PhantomData,
        }
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerInner
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        self.state.timers.set_timer(self.state, duration, handler)
    }

    pub fn exit(&self) {
        unsafe {
            NSApplication::sharedApplication().stop(None);
        }
    }
}
