use std::any::Any;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use objc::declare::ClassDecl;
use objc::runtime::{objc_autorelease, objc_disposeClassPair, objc_release, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use cocoa::appkit::{NSBackingStoreBuffered, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, BOOL, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};

use raw_window_handle::RawWindowHandle;

use super::app::{AppContextInner, AppState};
use super::OsError;
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, Point, Rect, Response, Result, WindowOptions,
};

const WINDOW_STATE: &str = "windowState";

fn class_name() -> String {
    use std::fmt::Write;

    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).unwrap();

    let mut name = "window-".to_string();
    for byte in bytes {
        write!(&mut name, "{:x}", byte).unwrap();
    }

    name
}

pub fn register_class() -> Result<*mut Class> {
    let name = class_name();
    let Some(mut decl) = ClassDecl::new(&name, class!(NSView)) else {
        return Err(Error::Os(OsError::Other("could not declare NSView subclass")));
    };

    decl.add_ivar::<*mut c_void>(WINDOW_STATE);

    unsafe {
        decl.add_method(
            sel!(windowShouldClose:),
            window_should_close as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    }

    Ok(decl.register() as *const Class as *mut Class)
}

pub unsafe fn unregister_class(class: *mut Class) {
    objc_disposeClassPair(class);
}

extern "C" fn window_should_close(this: &Object, _: Sel, _sender: id) -> BOOL {
    let state = unsafe { WindowState::from_view(this) };

    state.handle_event(Event::Close);

    NO
}

extern "C" fn dealloc(this: &Object, _: Sel) {
    unsafe {
        let state_ptr = *this.get_ivar::<*mut c_void>(WINDOW_STATE) as *const WindowState;
        drop(Rc::from_raw(state_ptr));

        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), dealloc];
    }
}

struct WindowState {
    app_state: Rc<AppState>,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>, Event) -> Response>>,
}

impl WindowState {
    unsafe fn from_view(view: *const Object) -> Rc<WindowState> {
        let state_ptr = *(*view).get_ivar::<*mut c_void>(WINDOW_STATE) as *const WindowState;

        let state_rc = Rc::from_raw(state_ptr);
        let state = Rc::clone(&state_rc);
        let _ = Rc::into_raw(state_rc);

        state
    }

    fn handle_event(&self, event: Event) -> Option<Response> {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            if let Ok(mut data) = self.app_state.data.try_borrow_mut() {
                if let Some(data) = &mut *data {
                    return Some(handler(&mut **data, &self.app_state, event));
                }
            }
        }

        None
    }
}

pub struct WindowInner {
    window: id,
    view: id,
    state: Rc<WindowState>,
}

impl WindowInner {
    pub fn open<T, H>(
        options: &WindowOptions,
        cx: &AppContext<T>,
        handler: H,
    ) -> Result<WindowInner>
    where
        T: 'static,
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        H: 'static,
    {
        unsafe {
            let rect = NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(options.rect.width, options.rect.height),
            );

            let style_mask = NSWindowStyleMask::NSTitledWindowMask
                | NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSMiniaturizableWindowMask
                | NSWindowStyleMask::NSResizableWindowMask;

            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                rect,
                style_mask,
                NSBackingStoreBuffered,
                NO,
            );

            let title = NSString::init_str(NSString::alloc(nil), &options.title);
            objc_autorelease(title);
            window.setTitle_(title);

            let view: id = msg_send![cx.inner.state.class, alloc];
            let view = view.initWithFrame_(rect);

            window.setDelegate_(view);
            window.setContentView_(view);
            window.center();

            let mut handler = handler;
            let handler_wrapper =
                move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                    let data = data_any.downcast_mut::<T>().unwrap();
                    let cx = AppContext::from_inner(AppContextInner::new(app_state));
                    handler(data, &cx, event)
                };

            let state = Rc::new(WindowState {
                app_state: Rc::clone(cx.inner.state),
                handler: RefCell::new(Box::new(handler_wrapper)),
            });

            let state_ptr = Rc::into_raw(Rc::clone(&state));
            (*view).set_ivar::<*mut c_void>(WINDOW_STATE, state_ptr as *mut c_void);

            Ok(WindowInner {
                window,
                view,
                state,
            })
        }
    }

    pub fn show(&self) {
        unsafe {
            self.window.orderFront_(nil);
        }
    }

    pub fn hide(&self) {
        unsafe {
            self.window.orderOut_(nil);
        }
    }

    pub fn present(&self, _bitmap: Bitmap) {}

    pub fn present_partial(&self, _bitmap: Bitmap, _rects: &[Rect]) {}

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, _position: Point) {}

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        unimplemented!()
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            self.window.close();
            objc_release(self.view);
        }
    }
}
