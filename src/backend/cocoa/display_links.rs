use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::rc::{Rc, Weak};

use objc2::rc::Id;

use icrate::ns_string;
use icrate::AppKit::NSScreen;
use icrate::Foundation::NSNumber;

use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation::runloop::*;

use super::app::AppState;
use super::ffi::display_link::*;
use super::window::View;
use crate::Event;

fn display_from_screen(screen: &NSScreen) -> Option<CGDirectDisplayID> {
    unsafe {
        let number = screen.deviceDescription().objectForKey(ns_string!("NSScreenNumber"))?;
        let id = Id::cast::<NSNumber>(number).unsignedIntegerValue() as CGDirectDisplayID;

        Some(id)
    }
}

fn display_from_view(view: &View) -> Option<CGDirectDisplayID> {
    let screen = unsafe { view.window()?.screen()? };
    display_from_screen(&*screen)
}

#[allow(non_snake_case)]
extern "C" fn callback(
    _displayLink: CVDisplayLinkRef,
    _inNow: *const CVTimeStamp,
    _inOutputTime: *const CVTimeStamp,
    _flagsIn: CVOptionFlags,
    _flagsOut: *mut CVOptionFlags,
    displayLinkContext: *mut c_void,
) -> CVReturn {
    let source = displayLinkContext as CFRunLoopSourceRef;
    unsafe {
        CFRunLoopSourceSignal(source);
        CFRunLoopWakeUp(CFRunLoopGetMain());
    }

    kCVReturnSuccess
}

extern "C" fn retain(info: *const c_void) -> *const c_void {
    unsafe {
        Rc::increment_strong_count(info as *const DisplayState);
    }

    info
}

extern "C" fn release(info: *const c_void) {
    unsafe {
        Rc::decrement_strong_count(info as *const DisplayState);
    }
}

extern "C" fn perform(info: *const c_void) {
    let state = unsafe { &*(info as *mut DisplayState) };

    if let Some(app_state) = state.app_state.upgrade() {
        let windows: Vec<*const View> = app_state.windows.borrow().keys().copied().collect();
        for view_ptr in windows {
            let view = app_state.windows.borrow().get(&view_ptr).map(|w| w.retain());
            if let Some(view) = view {
                if display_from_view(&*view) == Some(state.display_id) {
                    view.state().handle_event(Event::Frame);
                }
            }
        }
    }
}

struct DisplayState {
    display_id: CGDirectDisplayID,
    app_state: Weak<AppState>,
}

struct Display {
    link: CVDisplayLinkRef,
    source: CFRunLoopSourceRef,
}

impl Display {
    pub fn new(app_state: &Rc<AppState>, display_id: CGDirectDisplayID) -> Display {
        let state = Rc::new(DisplayState {
            display_id,
            app_state: Rc::downgrade(app_state),
        });

        let mut context = CFRunLoopSourceContext {
            version: 0,
            info: Rc::into_raw(state) as *mut c_void,
            retain: Some(retain),
            release: Some(release),
            copyDescription: None,
            equal: None,
            hash: None,
            schedule: None,
            cancel: None,
            perform,
        };

        let source = unsafe { CFRunLoopSourceCreate(ptr::null(), 0, &mut context) };
        unsafe {
            let run_loop = CFRunLoopGetMain();
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        }

        let mut link = ptr::null();
        unsafe {
            CVDisplayLinkCreateWithCGDisplay(display_id, &mut link);
            CVDisplayLinkSetOutputCallback(link, callback, source as *mut c_void);
            CVDisplayLinkStart(link);
        }

        Display { link, source }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            CVDisplayLinkStop(self.link);
            CVDisplayLinkRelease(self.link);

            CFRunLoopSourceInvalidate(self.source);
            CFRelease(self.source as CFTypeRef);
        }
    }
}

pub struct DisplayLinks {
    displays: RefCell<HashMap<CGDirectDisplayID, Display>>,
}

impl DisplayLinks {
    pub fn new() -> DisplayLinks {
        DisplayLinks {
            displays: RefCell::new(HashMap::new()),
        }
    }

    pub fn init(&self, app_state: &Rc<AppState>) {
        for screen in unsafe { NSScreen::screens() } {
            if let Some(id) = display_from_screen(&*screen) {
                self.displays.borrow_mut().insert(id, Display::new(app_state, id));
            }
        }
    }
}