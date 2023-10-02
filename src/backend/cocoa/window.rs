use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use objc2::declare::{ClassBuilder, Ivar, IvarEncode, IvarType};
use objc2::encode::Encoding;
use objc2::rc::{Allocated, Id};
use objc2::runtime::{AnyClass, Bool, ProtocolObject, Sel};
use objc2::{class, msg_send, msg_send_id, sel};
use objc2::{ClassType, Message, MessageReceiver, RefEncode};

use objc_sys::{objc_class, objc_disposeClassPair};

use icrate::AppKit::{
    NSCursor, NSEvent, NSScreen, NSTrackingArea, NSView, NSWindow, NSWindowDelegate,
};
use icrate::Foundation::{NSInteger, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::app::{AppContextInner, AppState};
use super::surface::Surface;
use super::OsError;
use crate::{
    AppContext, Bitmap, Cursor, Error, Event, MouseButton, Point, RawParent, Rect, Response,
    Result, Size, WindowOptions,
};

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

struct StateIvar;

unsafe impl IvarType for StateIvar {
    type Type = IvarEncode<Cell<*mut c_void>>;
    const NAME: &'static str = "windowState";
}

#[repr(C)]
pub struct View {
    superclass: NSView,
    state: Ivar<StateIvar>,
}

unsafe impl RefEncode for View {
    const ENCODING_REF: Encoding = NSView::ENCODING_REF;
}

unsafe impl Message for View {}

unsafe impl NSObjectProtocol for View {}
unsafe impl NSWindowDelegate for View {}

impl Deref for View {
    type Target = NSView;

    fn deref(&self) -> &Self::Target {
        &self.superclass
    }
}

impl DerefMut for View {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.superclass
    }
}

impl View {
    pub fn register_class() -> Result<&'static AnyClass> {
        let name = class_name();
        let Some(mut builder) = ClassBuilder::new(&name, class!(NSView)) else {
            return Err(Error::Os(OsError::Other("could not declare NSView subclass")));
        };

        builder.add_static_ivar::<StateIvar>();

        unsafe {
            builder.add_method(
                sel!(acceptsFirstMouse:),
                Self::accepts_first_mouse as unsafe extern "C" fn(_, _, _) -> _,
            );
            builder.add_method(
                sel!(isFlipped),
                Self::is_flipped as unsafe extern "C" fn(_, _) -> _,
            );
            builder.add_method(
                sel!(mouseMoved:),
                Self::mouse_moved as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(mouseDragged:),
                Self::mouse_moved as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(rightMouseDragged:),
                Self::mouse_moved as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(otherMouseDragged:),
                Self::mouse_moved as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(mouseDown:),
                Self::mouse_down as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(mouseUp:),
                Self::mouse_up as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(rightMouseDown:),
                Self::right_mouse_down as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(rightMouseUp:),
                Self::right_mouse_up as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(otherMouseDown:),
                Self::other_mouse_down as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(otherMouseUp:),
                Self::other_mouse_up as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(scrollWheel:),
                Self::scroll_wheel as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(cursorUpdate:),
                Self::cursor_update as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(windowShouldClose:),
                Self::window_should_close as unsafe extern "C" fn(_, _, _) -> _,
            );
            builder.add_method(sel!(dealloc), View::dealloc as unsafe extern "C" fn(_, _));
        }

        Ok(builder.register())
    }

    pub unsafe fn unregister_class(class: &'static AnyClass) {
        objc_disposeClassPair(class as *const _ as *mut objc_class);
    }

    fn state(&self) -> &WindowState {
        unsafe { &*(self.state.get() as *const WindowState) }
    }

    fn new(state: Box<WindowState>, frame: NSRect) -> Id<View> {
        let view: Option<Allocated<View>> = unsafe { msg_send_id![state.app_state.class, alloc] };
        let view: Id<View> = unsafe { msg_send_id![view, initWithFrame: frame] };

        view.state.set(Box::into_raw(state) as *mut c_void);

        view
    }

    unsafe extern "C" fn accepts_first_mouse(&self, _: Sel, _event: Option<&NSEvent>) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn is_flipped(&self, _: Sel) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn mouse_moved(&self, _: Sel, event: Option<&NSEvent>) {
        let Some(event) = event else { return; };

        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        self.state().handle_event(Event::MouseMove(Point {
            x: point.x,
            y: point.y,
        }));
    }

    unsafe extern "C" fn mouse_down(&self, _: Sel, event: Option<&NSEvent>) {
        let result = self.state().handle_event(Event::MouseDown(MouseButton::Left));

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), mouseDown: event];
        }
    }

    unsafe extern "C" fn mouse_up(&self, _: Sel, event: Option<&NSEvent>) {
        let result = self.state().handle_event(Event::MouseUp(MouseButton::Left));

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), mouseUp: event];
        }
    }

    unsafe extern "C" fn right_mouse_down(&self, _: Sel, event: Option<&NSEvent>) {
        let result = self.state().handle_event(Event::MouseDown(MouseButton::Right));

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), rightMouseDown: event];
        }
    }

    unsafe extern "C" fn right_mouse_up(&self, _: Sel, event: Option<&NSEvent>) {
        let result = self.state().handle_event(Event::MouseUp(MouseButton::Right));

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), rightMouseUp: event];
        }
    }

    unsafe extern "C" fn other_mouse_down(&self, _: Sel, event: Option<&NSEvent>) {
        let Some(event) = event else { return; };

        let button_number = event.buttonNumber();
        let result = if let Some(button) = mouse_button_from_number(button_number) {
            self.state().handle_event(Event::MouseDown(button))
        } else {
            None
        };

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), otherMouseDown: event];
        }
    }

    unsafe extern "C" fn other_mouse_up(&self, _: Sel, event: Option<&NSEvent>) {
        let Some(event) = event else { return; };

        let button_number = event.buttonNumber();
        let result = if let Some(button) = mouse_button_from_number(button_number) {
            self.state().handle_event(Event::MouseUp(button))
        } else {
            None
        };

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), otherMouseUp: event];
        }
    }

    unsafe extern "C" fn scroll_wheel(&self, _: Sel, event: Option<&NSEvent>) {
        let Some(event) = event else { return; };

        let dx = event.scrollingDeltaX();
        let dy = event.scrollingDeltaY();
        let delta = if event.hasPreciseScrollingDeltas() {
            Point::new(dx, dy)
        } else {
            Point::new(32.0 * dx, 32.0 * dy)
        };
        let result = self.state().handle_event(Event::Scroll(delta));

        if result != Some(Response::Capture) {
            let () = msg_send![super(self, NSView::class()), scrollWheel: event];
        }
    }

    unsafe extern "C" fn cursor_update(&self, _: Sel, _event: Option<&NSEvent>) {
        self.state().update_cursor();
    }

    unsafe extern "C" fn window_should_close(&self, _: Sel, _sender: &NSWindow) -> Bool {
        self.state().handle_event(Event::Close);

        Bool::NO
    }

    unsafe extern "C" fn dealloc(&self, _: Sel) {
        drop(Box::from_raw(self.state.get() as *mut WindowState));

        let () = msg_send![super(self, NSView::class()), dealloc];
    }
}

struct WindowState {
    surface: RefCell<Option<Surface>>,
    cursor: Cell<Cursor>,
    app_state: Rc<AppState>,
    handler: RefCell<Box<dyn FnMut(&mut dyn Any, &Rc<AppState>, Event) -> Response>>,
}

impl WindowState {
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

    fn update_cursor(&self) {
        fn try_get_cursor(selector: Sel) -> Id<NSCursor> {
            unsafe {
                let class = NSCursor::class();
                if objc2::msg_send![class, respondsToSelector: selector] {
                    let cursor: *mut NSCursor = class.send_message(selector, ());
                    if let Some(cursor) = Id::retain(cursor) {
                        return cursor;
                    }
                }

                NSCursor::arrowCursor()
            }
        }

        let cursor = self.cursor.get();

        let ns_cursor = match cursor {
            Cursor::Arrow => unsafe { NSCursor::arrowCursor() },
            Cursor::Crosshair => unsafe { NSCursor::crosshairCursor() },
            Cursor::Hand => unsafe { NSCursor::pointingHandCursor() },
            Cursor::IBeam => unsafe { NSCursor::IBeamCursor() },
            Cursor::No => unsafe { NSCursor::operationNotAllowedCursor() },
            Cursor::SizeNs => try_get_cursor(sel!(_windowResizeNorthSouthCursor)),
            Cursor::SizeWe => try_get_cursor(sel!(_windowResizeEastWestCursor)),
            Cursor::SizeNesw => try_get_cursor(sel!(_windowResizeNorthEastSouthWestCursor)),
            Cursor::SizeNwse => try_get_cursor(sel!(_windowResizeNorthWestSouthEastCursor)),
            Cursor::Wait => try_get_cursor(sel!(_waitCursor)),
            Cursor::None => self.app_state.empty_cursor.clone(),
        };

        unsafe {
            ns_cursor.set();
        }
    }
}

pub struct WindowInner {
    view: Id<View>,
    window: Option<Id<NSWindow>>,
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
        let parent_view = if let Some(parent) = options.parent {
            if let RawParent::Cocoa(parent_view) = parent {
                Some(parent_view as *const NSView)
            } else {
                return Err(Error::InvalidWindowHandle);
            }
        } else {
            None
        };

        let origin = if options.parent.is_some() {
            Point::new(0.0, 0.0)
        } else {
            options.position.unwrap_or(Point::new(0.0, 0.0))
        };
        let frame = NSRect::new(
            NSPoint::new(origin.x, origin.y),
            NSSize::new(options.size.width, options.size.height),
        );

        let mut handler = handler;
        let handler_wrapper =
            move |data_any: &mut dyn Any, app_state: &Rc<AppState>, event: Event<'_>| {
                let data = data_any.downcast_mut::<T>().unwrap();
                let cx = AppContext::from_inner(AppContextInner::new(app_state));
                handler(data, &cx, event)
            };

        let state = Box::new(WindowState {
            surface: RefCell::new(None),
            cursor: Cell::new(Cursor::Arrow),
            app_state: Rc::clone(cx.inner.state),
            handler: RefCell::new(Box::new(handler_wrapper)),
        });
        let view = View::new(state, frame);

        unsafe {
            let tracking_options = icrate::AppKit::NSTrackingMouseEnteredAndExited
                | icrate::AppKit::NSTrackingMouseMoved
                | icrate::AppKit::NSTrackingCursorUpdate
                | icrate::AppKit::NSTrackingActiveAlways
                | icrate::AppKit::NSTrackingInVisibleRect
                | icrate::AppKit::NSTrackingEnabledDuringMouseDrag;

            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
                tracking_options,
                Some(&view),
                None,
            );
            view.addTrackingArea(&tracking_area);

            if let Some(parent_view) = parent_view {
                view.setHidden(true);
                (*parent_view).addSubview(&view);
            }

            let window = if parent_view.is_none() {
                let origin = options.position.unwrap_or(Point::new(0.0, 0.0));
                let content_rect = NSRect::new(
                    NSPoint::new(origin.x, origin.y),
                    NSSize::new(options.size.width, options.size.height),
                );

                let style_mask = icrate::AppKit::NSWindowStyleMaskTitled
                    | icrate::AppKit::NSWindowStyleMaskClosable
                    | icrate::AppKit::NSWindowStyleMaskMiniaturizable
                    | icrate::AppKit::NSWindowStyleMaskResizable;

                let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(),
                    content_rect,
                    style_mask,
                    icrate::AppKit::NSBackingStoreBuffered,
                    false,
                );

                window.setReleasedWhenClosed(false);

                window.setTitle(&NSString::from_str(&options.title));

                let delegate = ProtocolObject::<dyn NSWindowDelegate>::from_ref(&*view);
                window.setDelegate(Some(&delegate));
                window.setContentView(Some(&view));

                if options.position.is_none() {
                    window.center();
                }

                Some(window)
            } else {
                None
            };

            let inner = WindowInner { view, window };

            let scale = inner.scale();

            let surface = Surface::new(
                (scale * options.size.width).round() as usize,
                (scale * options.size.height).round() as usize,
            );

            let () = msg_send![&inner.view, setLayer: &*surface.layer];
            inner.view.setWantsLayer(true);

            surface.layer.setContentsScale(scale);

            inner.view.state().surface.replace(Some(surface));

            Ok(inner)
        }
    }

    pub fn show(&self) {
        unsafe {
            if let Some(window) = &self.window {
                window.orderFront(None);
            } else {
                self.view.setHidden(false);
            }
        }
    }

    pub fn hide(&self) {
        unsafe {
            if let Some(window) = &self.window {
                window.orderOut(None);
            } else {
                self.view.setHidden(true);
            }
        }
    }

    pub fn size(&self) -> Size {
        let frame = unsafe { self.view.frame() };

        Size::new(frame.size.width, frame.size.height)
    }

    pub fn scale(&self) -> f64 {
        unsafe {
            if let Some(window) = self.view.window() {
                window.backingScaleFactor()
            } else if let Some(screen) = NSScreen::screens().get(0) {
                screen.backingScaleFactor()
            } else {
                1.0
            }
        }
    }

    pub fn present(&self, bitmap: Bitmap) {
        if let Some(surface) = &mut *self.view.state().surface.borrow_mut() {
            let width = surface.width;
            let height = surface.height;
            let copy_width = bitmap.width().min(width);
            let copy_height = bitmap.height().min(height);

            surface.with_buffer(|buffer| {
                for row in 0..copy_height {
                    let src =
                        &bitmap.data()[row * bitmap.width()..row * bitmap.width() + copy_width];
                    let dst = &mut buffer[row * width..row * width + copy_width];
                    dst.copy_from_slice(src);
                }
            });

            surface.present();
        }
    }

    pub fn present_partial(&self, bitmap: Bitmap, _rects: &[Rect]) {
        self.present(bitmap);
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        let state = self.view.state();

        state.cursor.set(cursor);
        state.update_cursor();
    }

    pub fn set_mouse_position(&self, _position: Point) {}
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        unsafe {
            if let Some(window) = &self.window {
                window.close();
            } else {
                self.view.removeFromSuperview();
            }
        }
    }
}
