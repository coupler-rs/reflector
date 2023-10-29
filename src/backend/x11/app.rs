use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Rc;
use std::time::{Duration, Instant};

use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::xproto::{Button, ConnectionExt as _, Window};
use x11rb::protocol::{shm, xproto};
use x11rb::rust_connection::RustConnection;
use x11rb::{cursor, protocol, resource_manager};

use super::timer::{TimerInner, Timers};
use super::window::WindowState;
use crate::{AppContext, AppOptions, Cursor, Event, MouseButton, Point, Rect, Result};

fn mouse_button_from_code(code: Button) -> Option<MouseButton> {
    match code {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        8 => Some(MouseButton::Back),
        9 => Some(MouseButton::Forward),
        _ => None,
    }
}

fn scroll_delta_from_code(code: Button) -> Option<Point> {
    match code {
        4 => Some(Point::new(0.0, 1.0)),
        5 => Some(Point::new(0.0, -1.0)),
        6 => Some(Point::new(-1.0, 0.0)),
        7 => Some(Point::new(1.0, 0.0)),
        _ => None,
    }
}

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        _NET_WM_NAME,
        UTF8_STRING,
    }
}

pub struct AppState {
    pub connection: RustConnection,
    pub screen_index: usize,
    pub atoms: Atoms,
    pub shm_supported: bool,
    pub resources: resource_manager::Database,
    pub cursor_handle: cursor::Handle,
    pub cursor_cache: RefCell<HashMap<Cursor, xproto::Cursor>>,
    pub scale: f64,
    pub running: Cell<bool>,
    pub windows: RefCell<HashMap<Window, Rc<WindowState>>>,
    pub timers: Timers,
}

impl Drop for AppState {
    fn drop(&mut self) {
        for (_, cursor) in self.cursor_cache.take() {
            let _ = self.connection.free_cursor(cursor);
        }
        let _ = self.connection.flush();
    }
}

pub struct AppInner<T> {
    state: Rc<AppState>,
    data: T,
}

impl<T: 'static> AppInner<T> {
    pub fn new<F>(_options: &AppOptions, build: F) -> Result<AppInner<T>>
    where
        F: FnOnce(&AppContext<T>) -> Result<T>,
        T: 'static,
    {
        let (connection, screen_index) = x11rb::connect(None)?;
        let atoms = Atoms::new(&connection)?.reply()?;
        let shm_supported = connection.extension_information(shm::X11_EXTENSION_NAME)?.is_some();
        let resources = resource_manager::new_from_default(&connection)?;
        let cursor_handle = cursor::Handle::new(&connection, screen_index, &resources)?.reply()?;

        let scale = if let Ok(Some(dpi)) = resources.get_value::<u32>("Xft.dpi", "") {
            dbg!(dpi);
            dpi as f64 / 96.0
        } else {
            1.0
        };

        let state = Rc::new(AppState {
            connection,
            screen_index,
            shm_supported,
            atoms,
            resources,
            cursor_handle,
            cursor_cache: RefCell::new(HashMap::new()),
            scale,
            running: Cell::new(false),
            windows: RefCell::new(HashMap::new()),
            timers: Timers::new(),
        });

        let cx = AppContext::from_inner(AppContextInner::new(&state));
        let data = build(&cx)?;

        let inner = AppInner { state, data };

        Ok(inner)
    }

    pub fn run(&mut self) -> Result<()> {
        self.state.running.set(true);

        let fd = self.as_raw_fd();

        while self.state.running.get() {
            self.drain_events()?;
            self.state.timers.poll(&mut self.data, &self.state);
            self.drain_events()?;

            if !self.state.running.get() {
                break;
            }

            let mut fds = [libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            }];

            let timeout = if let Some(next_time) = self.state.timers.next_time() {
                let duration = next_time.saturating_duration_since(Instant::now());
                duration.as_millis() as i32
            } else {
                -1
            };

            unsafe {
                libc::poll(fds.as_mut_ptr(), fds.len() as u64, timeout);
            }
        }

        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        self.drain_events()?;
        self.state.timers.poll(&mut self.data, &self.state);
        self.drain_events()?;

        Ok(())
    }

    fn drain_events(&mut self) -> Result<()> {
        while let Some(event) = self.state.connection.poll_for_event()? {
            match event {
                protocol::Event::Expose(event) => {
                    let window = self.state.windows.borrow().get(&event.window).cloned();
                    if let Some(window) = window {
                        window.expose_rects.borrow_mut().push(Rect {
                            x: event.x as f64,
                            y: event.y as f64,
                            width: event.width as f64,
                            height: event.height as f64,
                        });

                        if event.count == 0 {
                            let rects = window.expose_rects.take();
                            window.handler.borrow_mut()(
                                &mut self.data,
                                &self.state,
                                Event::Expose(&rects),
                            );
                        }
                    }
                }
                protocol::Event::ClientMessage(event) => {
                    if event.format == 32
                        && event.data.as_data32()[0] == self.state.atoms.WM_DELETE_WINDOW
                    {
                        let window = self.state.windows.borrow().get(&event.window).cloned();
                        if let Some(window) = window {
                            window.handler.borrow_mut()(&mut self.data, &self.state, Event::Close);
                        }
                    }
                }
                protocol::Event::MotionNotify(event) => {
                    let window = self.state.windows.borrow().get(&event.event).cloned();
                    if let Some(window) = window {
                        let point = Point {
                            x: event.event_x as f64,
                            y: event.event_y as f64,
                        };

                        window.handler.borrow_mut()(
                            &mut self.data,
                            &self.state,
                            Event::MouseMove(point),
                        );
                    }
                }
                protocol::Event::ButtonPress(event) => {
                    let window = self.state.windows.borrow().get(&event.event).cloned();
                    if let Some(window) = window {
                        if let Some(button) = mouse_button_from_code(event.detail) {
                            window.handler.borrow_mut()(
                                &mut self.data,
                                &self.state,
                                Event::MouseDown(button),
                            );
                        } else if let Some(delta) = scroll_delta_from_code(event.detail) {
                            window.handler.borrow_mut()(
                                &mut self.data,
                                &self.state,
                                Event::Scroll(delta),
                            );
                        }
                    }
                }
                protocol::Event::ButtonRelease(event) => {
                    let window = self.state.windows.borrow().get(&event.event).cloned();
                    if let Some(window) = window {
                        if let Some(button) = mouse_button_from_code(event.detail) {
                            window.handler.borrow_mut()(
                                &mut self.data,
                                &self.state,
                                Event::MouseUp(button),
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl<T> Drop for AppInner<T> {
    fn drop(&mut self) {
        for window_state in self.state.windows.take().into_values() {
            window_state.close();
        }
        let _ = self.state.connection.flush();
    }
}

impl<T> AsRawFd for AppInner<T> {
    fn as_raw_fd(&self) -> RawFd {
        self.state.connection.stream().as_raw_fd()
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
        self.state.running.set(false);
    }
}
