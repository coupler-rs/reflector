use std::any::Any;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use objc::declare::ClassDecl;
use objc::runtime::{objc_autorelease, objc_disposeClassPair, objc_release, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use cocoa::appkit::{NSBackingStoreBuffered, NSEvent, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSInteger, NSPoint, NSRect, NSSize, NSString, NSUInteger};

use super::app::{AppContextInner, AppState};
use super::surface::Surface;
use super::OsError;
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, MouseButton, Point, RawParent, Rect, Response,
    Result, Size, WindowOptions,
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
            sel!(acceptsFirstMouse:),
            accepts_first_mouse as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(
            sel!(isFlipped),
            is_flipped as extern "C" fn(&Object, Sel) -> BOOL,
        );
        decl.add_method(
            sel!(mouseMoved:),
            mouse_moved as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseDragged:),
            mouse_moved as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseDragged:),
            mouse_moved as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDragged:),
            mouse_moved as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseDown:),
            mouse_down as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(sel!(mouseUp:), mouse_up as extern "C" fn(&Object, Sel, id));
        decl.add_method(
            sel!(rightMouseDown:),
            right_mouse_down as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseUp:),
            right_mouse_up as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDown:),
            other_mouse_down as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseUp:),
            other_mouse_up as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(scrollWheel:),
            scroll_wheel as extern "C" fn(&Object, Sel, id),
        );
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

extern "C" fn accepts_first_mouse(_this: &Object, _: Sel, _event: id) -> BOOL {
    YES
}

extern "C" fn is_flipped(_this: &Object, _: Sel) -> BOOL {
    YES
}

extern "C" fn mouse_moved(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let this = this as *const Object as id;
    let point = unsafe { this.convertPoint_fromView_(event.locationInWindow(), nil) };
    state.handle_event(Event::MouseMove(Point {
        x: point.x,
        y: point.y,
    }));
}

extern "C" fn mouse_down(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let result = state.handle_event(Event::MouseDown(MouseButton::Left));

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), mouseDown: event];
        }
    }
}

extern "C" fn mouse_up(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let result = state.handle_event(Event::MouseUp(MouseButton::Left));

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), mouseUp: event];
        }
    }
}

extern "C" fn right_mouse_down(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let result = state.handle_event(Event::MouseDown(MouseButton::Right));

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), rightMouseDown: event];
        }
    }
}

extern "C" fn right_mouse_up(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let result = state.handle_event(Event::MouseUp(MouseButton::Right));

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), rightMouseUp: event];
        }
    }
}

fn mouse_button_from_number(button_number: NSInteger) -> Option<MouseButton> {
    match button_number {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Right),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Back),
        4 => Some(MouseButton::Forward),
        _ => None,
    }
}

extern "C" fn other_mouse_down(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let button_number = unsafe { event.buttonNumber() };
    let result = if let Some(button) = mouse_button_from_number(button_number) {
        state.handle_event(Event::MouseDown(button))
    } else {
        None
    };

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), otherMouseDown: event];
        }
    }
}

extern "C" fn other_mouse_up(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let button_number = unsafe { event.buttonNumber() };
    let result = if let Some(button) = mouse_button_from_number(button_number) {
        state.handle_event(Event::MouseUp(button))
    } else {
        None
    };

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), otherMouseUp: event];
        }
    }
}

extern "C" fn scroll_wheel(this: &Object, _: Sel, event: id) {
    let state = unsafe { WindowState::from_view(this) };

    let dx = unsafe { event.scrollingDeltaX() };
    let dy = unsafe { event.scrollingDeltaY() };
    let delta = if unsafe { event.hasPreciseScrollingDeltas() } == YES {
        Point::new(dx, dy)
    } else {
        Point::new(32.0 * dx, 32.0 * dy)
    };
    let result = state.handle_event(Event::Scroll(delta));

    if result != Some(Response::Capture) {
        unsafe {
            let superclass = msg_send![this, superclass];
            let () = msg_send![super(this, superclass), scrollWheel: event];
        }
    }
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
    surface: RefCell<Surface>,
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
    view: id,
    window: Option<id>,
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

            let parent_view = if let Some(parent) = options.parent {
                if let RawParent::Cocoa(parent_view) = parent {
                    Some(parent_view as id)
                } else {
                    return Err(Error::InvalidWindowHandle);
                }
            } else {
                None
            };

            let view: id = msg_send![cx.inner.state.class, alloc];
            let view = view.initWithFrame_(rect);

            let scale = view.backingScaleFactor();

            let surface = Surface::new(
                (scale * options.rect.width) as usize,
                (scale * options.rect.height) as usize,
            );

            view.setLayer(surface.layer.id());
            view.setWantsLayer(YES);

            surface.layer.set_contents_scale(scale);

            #[allow(non_upper_case_globals)]
            let tracking_options = {
                const NSTrackingMouseEnteredAndExited: NSUInteger = 0x1;
                const NSTrackingMouseMoved: NSUInteger = 0x2;
                const NSTrackingCursorUpdate: NSUInteger = 0x4;
                const NSTrackingActiveAlways: NSUInteger = 0x80;
                const NSTrackingInVisibleRect: NSUInteger = 0x200;
                const NSTrackingEnabledDuringMouseDrag: NSUInteger = 0x400;

                NSTrackingMouseEnteredAndExited
                    | NSTrackingMouseMoved
                    | NSTrackingCursorUpdate
                    | NSTrackingActiveAlways
                    | NSTrackingInVisibleRect
                    | NSTrackingEnabledDuringMouseDrag
            };

            let tracking_area: id = msg_send![class!(NSTrackingArea), alloc];
            let tracking_area: id = msg_send![
                tracking_area,
                initWithRect: NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(0.0, 0.0),
                )
                options: tracking_options
                owner: view
                userInfo: nil
            ];
            let () = msg_send![view, addTrackingArea: tracking_area];
            objc_autorelease(tracking_area);

            let mut handler = handler;
            let handler_wrapper =
                move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                    let data = data_any.downcast_mut::<T>().unwrap();
                    let cx = AppContext::from_inner(AppContextInner::new(app_state));
                    handler(data, &cx, event)
                };

            let state = Rc::new(WindowState {
                surface: RefCell::new(surface),
                app_state: Rc::clone(cx.inner.state),
                handler: RefCell::new(Box::new(handler_wrapper)),
            });

            let state_ptr = Rc::into_raw(Rc::clone(&state));
            (*view).set_ivar::<*mut c_void>(WINDOW_STATE, state_ptr as *mut c_void);

            if let Some(parent_view) = parent_view {
                let () = msg_send![view, setHidden: YES];
                parent_view.addSubview_(view);
            }

            let window = if parent_view.is_none() {
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

                window.setDelegate_(view);
                window.setContentView_(view);
                window.center();

                Some(window)
            } else {
                None
            };

            Ok(WindowInner {
                view,
                window,
                state,
            })
        }
    }

    pub fn show(&self) {
        unsafe {
            if let Some(window) = self.window {
                window.orderFront_(nil);
            } else {
                let () = msg_send![self.view, setHidden: NO];
            }
        }
    }

    pub fn hide(&self) {
        unsafe {
            if let Some(window) = self.window {
                window.orderOut_(nil);
            } else {
                let () = msg_send![self.view, setHidden: YES];
            }
        }
    }

    pub fn size(&self) -> Size {
        let frame = unsafe { NSView::frame(self.view) };

        Size::new(frame.size.width, frame.size.height)
    }

    pub fn scale(&self) -> f64 {
        unsafe { self.view.backingScaleFactor() }
    }

    pub fn present(&self, bitmap: Bitmap) {
        let mut surface = self.state.surface.borrow_mut();

        let width = surface.width;
        let height = surface.height;
        let copy_width = bitmap.width().min(width);
        let copy_height = bitmap.height().min(height);

        surface.with_buffer(|buffer| {
            for row in 0..copy_height {
                let src = &bitmap.data()[row * bitmap.width()..row * bitmap.width() + copy_width];
                let dst = &mut buffer[row * width..row * width + copy_width];
                dst.copy_from_slice(src);
            }
        });

        unsafe {
            let () = msg_send![surface.layer.id(), setContentsChanged];
        }
    }

    pub fn present_partial(&self, bitmap: Bitmap, _rects: &[Rect]) {
        self.present(bitmap);
    }

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, _position: Point) {}
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            if let Some(window) = self.window {
                window.close();
            }

            objc_release(self.view);
        }
    }
}
