use raw_window_handle::RawWindowHandle;

use crate::{AppContext, Bitmap, Cursor, Event, Point, Rect, Response, Result, WindowOptions};

pub struct WindowInner {}

impl WindowInner {
    pub fn open<T, H>(
        _options: &WindowOptions,
        _cx: &AppContext<T>,
        _handler: H,
    ) -> Result<WindowInner>
    where
        H: FnMut(&mut T, &AppContext<T>, Event) -> Response,
        H: 'static,
    {
        Ok(WindowInner {})
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
