use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::result;
use std::time::Duration;

use objc::rc::autoreleasepool;
use objc::runtime::{objc_release, Class};
use objc::{msg_send, sel, sel_impl};

use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSCursor, NSImage,
};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSPoint, NSSize};

use super::timer::{TimerHandleInner, Timers};
use super::window::{register_class, unregister_class};
use crate::{App, AppContext, AppMode, AppOptions, Error, IntoInnerError, Result};

pub struct AppState {
    pub class: *mut Class,
    pub empty_cursor: id,
    pub data: RefCell<Option<Box<dyn Any>>>,
    pub timer_state: Timers,
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            objc_release(self.empty_cursor);
            unregister_class(self.class);
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
        autoreleasepool(|| {
            let class = register_class()?;

            let empty_cursor = unsafe {
                let empty_cursor_image =
                    NSImage::initWithSize_(NSImage::alloc(nil), NSSize::new(1.0, 1.0));
                let empty_cursor: id = msg_send![
                    NSCursor::alloc(nil),
                    initWithImage: empty_cursor_image
                    hotSpot: NSPoint::new(0.0, 0.0)
                ];
                objc_release(empty_cursor_image);

                empty_cursor
            };

            let state = Rc::new(AppState {
                class,
                empty_cursor,
                data: RefCell::new(None),
                timer_state: Timers::new(),
            });

            let cx = AppContext::from_inner(AppContextInner {
                state: &state,
                _marker: PhantomData,
            });
            let data = build(&cx)?;

            state.data.replace(Some(Box::new(data)));

            if options.mode == AppMode::Owner {
                unsafe {
                    let app = NSApp();
                    app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
                    app.activateIgnoringOtherApps_(YES);
                }
            }

            Ok(AppInner {
                state,
                _marker: PhantomData,
            })
        })
    }

    pub fn run(&mut self) -> Result<()> {
        autoreleasepool(|| unsafe {
            let app = NSApp();
            app.run();

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
        autoreleasepool(|| {
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

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerHandleInner
    where
        H: 'static,
        H: FnMut(&mut T, &AppContext<T>),
    {
        self.state.timer_state.set_timer(self.state, duration, handler)
    }

    pub fn exit(&self) {
        unsafe {
            NSApp().stop_(nil);
        }
    }
}
