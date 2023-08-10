use objc::runtime::objc_release;

use cocoa::appkit::{NSBackingStoreBuffered, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize};

use raw_window_handle::RawWindowHandle;

use crate::{AppContext, Bitmap, Cursor, Event, Point, Rect, Response, Result, WindowOptions};

pub struct WindowInner {
    window: id,
    view: id,
}

impl WindowInner {
    pub fn open<T, H>(
        options: &WindowOptions,
        _cx: &AppContext<T>,
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

            let view = NSView::alloc(nil).initWithFrame_(rect);

            window.setContentView_(view);
            window.center();
            window.makeKeyAndOrderFront_(nil);

            Ok(WindowInner { window, view })
        }
    }

    pub fn show(&self) {}

    pub fn hide(&self) {}

    pub fn present(&self, bitmap: Bitmap) {}

    pub fn present_partial(&self, bitmap: Bitmap, rects: &[Rect]) {}

    pub fn set_cursor(&self, _cursor: Cursor) {}

    pub fn set_mouse_position(&self, position: Point) {}

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
