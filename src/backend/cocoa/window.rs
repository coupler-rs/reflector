use std::ffi::c_void;

use objc::declare::ClassDecl;
use objc::runtime::{objc_disposeClassPair, objc_release, Class};
use objc::{class, msg_send, sel, sel_impl};

use cocoa::appkit::{NSBackingStoreBuffered, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize};

use raw_window_handle::RawWindowHandle;

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

    Ok(decl.register() as *const Class as *mut Class)
}

pub unsafe fn unregister_class(class: *mut Class) {
    objc_disposeClassPair(class);
}

pub struct WindowInner {
    window: id,
    view: id,
}

impl WindowInner {
    pub fn open<T, H>(
        options: &WindowOptions,
        cx: &AppContext<T>,
        _handler: H,
    ) -> Result<WindowInner>
    where
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

            let view: id = msg_send![cx.inner.state.class, alloc];
            let view = view.initWithFrame_(rect);

            window.setContentView_(view);
            window.center();
            window.makeKeyAndOrderFront_(nil);

            Ok(WindowInner { window, view })
        }
    }

    pub fn show(&self) {}

    pub fn hide(&self) {}

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
