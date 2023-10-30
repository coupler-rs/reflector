use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;
use std::{mem, ptr};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{HBRUSH, HMONITOR};
use windows::Win32::UI::WindowsAndMessaging::{
    self as msg, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW,
    TranslateMessage, UnregisterClassW, HCURSOR, HICON, HMENU, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WNDCLASSW, WNDCLASS_STYLES,
};

use super::dpi::DpiFns;
use super::timer::{TimerInner, Timers};
use super::vsync::VsyncThreads;
use super::window::{self, WindowState};
use super::{class_name, hinstance, to_wstring, WM_USER_VBLANK};
use crate::{AppMode, AppOptions, Result, TimerContext};

fn register_message_class() -> Result<PCWSTR> {
    let class_name = to_wstring(&class_name("message-"));

    let wnd_class = WNDCLASSW {
        style: WNDCLASS_STYLES(0),
        lpfnWndProc: Some(message_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance(),
        hIcon: HICON(0),
        hCursor: HCURSOR(0),
        hbrBackground: HBRUSH(0),
        lpszMenuName: PCWSTR(ptr::null()),
        lpszClassName: PCWSTR(class_name.as_ptr()),
    };

    let class = unsafe { RegisterClassW(&wnd_class) };
    if class == 0 {
        return Err(windows::core::Error::from_win32().into());
    }

    Ok(PCWSTR(class as *const u16))
}

unsafe fn unregister_message_class(class: PCWSTR) {
    let _ = UnregisterClassW(class, hinstance());
}

pub unsafe extern "system" fn message_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let app_state_ptr = GetWindowLongPtrW(hwnd, msg::GWLP_USERDATA) as *mut AppState;
    if !app_state_ptr.is_null() {
        let app_state_weak = Weak::from_raw(app_state_ptr);
        let app_state = app_state_weak.clone();
        let _ = app_state_weak.into_raw();

        match msg {
            msg::WM_TIMER => {
                if let Some(app_state) = app_state.upgrade() {
                    app_state.timers.handle_timer(&app_state, wparam.0);
                }
            }
            WM_USER_VBLANK => {
                if let Some(app_state) = app_state.upgrade() {
                    app_state.vsync_threads.handle_vblank(&app_state, HMONITOR(lparam.0));
                }
            }
            msg::WM_DESTROY => {
                drop(Weak::from_raw(app_state_ptr));
                SetWindowLongPtrW(hwnd, msg::GWLP_USERDATA, 0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub struct AppState {
    pub message_class: PCWSTR,
    pub message_hwnd: HWND,
    pub window_class: PCWSTR,
    pub dpi: DpiFns,
    pub timers: Timers,
    pub vsync_threads: VsyncThreads,
    pub windows: RefCell<HashMap<isize, Rc<WindowState>>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        self.vsync_threads.join_all();

        unsafe {
            window::unregister_class(self.window_class);

            let _ = DestroyWindow(self.message_hwnd);
            unregister_message_class(self.message_class);
        }
    }
}

pub struct AppInner {
    pub state: Rc<AppState>,
}

impl AppInner {
    pub fn new(options: &AppOptions) -> Result<AppInner> {
        let message_class = register_message_class()?;

        let message_hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                message_class,
                PCWSTR(ptr::null()),
                WINDOW_STYLE(0),
                msg::CW_USEDEFAULT,
                msg::CW_USEDEFAULT,
                0,
                0,
                HWND(0),
                HMENU(0),
                hinstance(),
                None,
            )
        };
        if message_hwnd == HWND(0) {
            return Err(windows::core::Error::from_win32().into());
        }

        let window_class = window::register_class()?;

        let dpi = DpiFns::load();
        if options.mode == AppMode::Owner {
            dpi.set_dpi_aware();
        }

        let timers = Timers::new();

        let vsync_threads = VsyncThreads::new();

        let state = Rc::new(AppState {
            message_class,
            message_hwnd,
            window_class,
            dpi,
            timers,
            vsync_threads,
            windows: RefCell::new(HashMap::new()),
        });

        let state_ptr = Weak::into_raw(Rc::downgrade(&state));
        unsafe {
            SetWindowLongPtrW(message_hwnd, msg::GWLP_USERDATA, state_ptr as isize);
        }

        state.vsync_threads.init(&state);

        Ok(AppInner { state })
    }

    pub fn set_timer<H>(&self, duration: Duration, handler: H) -> TimerInner
    where
        H: FnMut(&TimerContext) + 'static,
    {
        self.state.timers.set_timer(&self.state, duration, handler)
    }

    pub fn run(&self) -> Result<()> {
        loop {
            unsafe {
                let mut msg: MSG = mem::zeroed();

                let result = GetMessageW(&mut msg, HWND(0), 0, 0);
                if result.0 < 0 {
                    return Err(windows::core::Error::from_win32().into());
                } else if result.0 == 0 {
                    return Ok(());
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn exit(&self) {
        unsafe {
            PostQuitMessage(0);
        }
    }

    pub fn poll(&self) -> Result<()> {
        loop {
            unsafe {
                let mut msg: MSG = mem::zeroed();

                let result = PeekMessageW(&mut msg, HWND(0), 0, 0, msg::PM_REMOVE);
                if result.0 == 0 {
                    return Ok(());
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn shutdown(&self) {
        for window_state in self.state.windows.take().into_values() {
            window_state.close();
        }

        self.state.timers.shutdown();
    }
}
