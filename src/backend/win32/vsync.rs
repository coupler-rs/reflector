use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory, IDXGIFactory, IDXGIOutput, DXGI_OUTPUT_DESC,
};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use super::app::AppState;
use super::WM_USER_VBLANK;
use crate::Event;

struct ThreadState {
    pending: AtomicBool,
    halt: AtomicBool,
}

struct Thread {
    state: Arc<ThreadState>,
    handle: JoinHandle<()>,
}

impl Thread {
    fn new(message_hwnd: HWND, output: IDXGIOutput, monitor: HMONITOR) -> Thread {
        let state = Arc::new(ThreadState {
            pending: AtomicBool::new(false),
            halt: AtomicBool::new(false),
        });

        let handle = thread::spawn({
            let state = state.clone();
            move || {
                while !state.halt.load(Ordering::Relaxed) {
                    unsafe {
                        if output.WaitForVBlank().is_err() {
                            return;
                        }

                        state.pending.store(true, Ordering::Relaxed);
                        let _ = PostMessageW(
                            message_hwnd,
                            WM_USER_VBLANK,
                            WPARAM(0),
                            LPARAM(monitor.0),
                        );
                    }
                }
            }
        });

        Thread { state, handle }
    }
}

pub struct VsyncThreads {
    threads: RefCell<HashMap<isize, Thread>>,
}

impl VsyncThreads {
    pub fn new() -> VsyncThreads {
        VsyncThreads {
            threads: RefCell::new(HashMap::new()),
        }
    }

    pub fn init(&self, app_state: &AppState) {
        let factory = unsafe { CreateDXGIFactory::<IDXGIFactory>() }.unwrap();

        let mut i = 0;
        while let Ok(adapter) = unsafe { factory.EnumAdapters(i) } {
            i += 1;

            let mut j = 0;
            while let Ok(output) = unsafe { adapter.EnumOutputs(j) } {
                j += 1;

                let mut desc = DXGI_OUTPUT_DESC::default();
                unsafe {
                    output.GetDesc(&mut desc).unwrap();
                }

                let thread = Thread::new(app_state.message_hwnd, output, desc.Monitor);
                self.threads.borrow_mut().insert(desc.Monitor.0, thread);
            }
        }
    }

    pub fn handle_vblank(&self, app_state: &Rc<AppState>, monitor: HMONITOR) {
        let was_pending = if let Some(thread) = self.threads.borrow().get(&monitor.0) {
            thread.state.pending.swap(false, Ordering::Relaxed)
        } else {
            false
        };

        // If we've received multiple vblank messages for this monitor since the last time we
        // handled one, only handle the first one.
        if !was_pending {
            return;
        }

        let windows: Vec<isize> = app_state.windows.borrow().keys().copied().collect();
        for hwnd in windows {
            let window_monitor = unsafe { MonitorFromWindow(HWND(hwnd), MONITOR_DEFAULTTONEAREST) };
            if window_monitor == monitor {
                let window = app_state.windows.borrow().get(&hwnd).cloned();
                if let Some(window) = window {
                    window.handle_event(Event::Frame);
                }
            }
        }
    }

    pub fn join_all(&self) {
        for (_, thread) in &*self.threads.borrow() {
            thread.state.halt.store(true, Ordering::Relaxed);
        }

        for (_, thread) in self.threads.take() {
            let _ = thread.handle.join();
        }
    }
}
