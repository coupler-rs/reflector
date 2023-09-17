use std::mem;

use windows_sys::core::HRESULT;
use windows_sys::Win32::Foundation::{BOOL, HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::HMONITOR;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, MONITOR_DPI_TYPE, PROCESS_DPI_AWARENESS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{WINDOW_EX_STYLE, WINDOW_STYLE};

macro_rules! c_str {
    ($str:literal) => {
        concat!($str, "\0").as_ptr()
    };
}

#[allow(non_snake_case)]
pub struct DpiFns {
    pub SetProcessDPIAware: Option<unsafe extern "system" fn() -> BOOL>,
    pub SetProcessDpiAwareness:
        Option<unsafe extern "system" fn(value: PROCESS_DPI_AWARENESS) -> HRESULT>,
    pub SetProcessDpiAwarenessContext:
        Option<unsafe extern "system" fn(value: DPI_AWARENESS_CONTEXT) -> BOOL>,
    pub GetDpiForMonitor: Option<
        unsafe extern "system" fn(
            hmonitor: HMONITOR,
            dpitype: MONITOR_DPI_TYPE,
            dpix: *mut u32,
            dpiy: *mut u32,
        ) -> HRESULT,
    >,
    pub GetDpiForWindow: Option<unsafe extern "system" fn(hwnd: HWND) -> u32>,
    pub EnableNonClientDpiScaling: Option<unsafe extern "system" fn(hwnd: HWND) -> BOOL>,
    pub AdjustWindowRectExForDpi: Option<
        unsafe extern "system" fn(
            lprect: *mut RECT,
            dwstyle: WINDOW_STYLE,
            bmenu: BOOL,
            dwexstyle: WINDOW_EX_STYLE,
            dpi: u32,
        ) -> BOOL,
    >,
}

impl DpiFns {
    pub fn load() -> DpiFns {
        macro_rules! load {
            ($lib:expr, $symbol:literal) => {
                if $lib != 0 {
                    mem::transmute(GetProcAddress($lib, c_str!($symbol)))
                } else {
                    None
                }
            };
        }

        unsafe {
            let user32 = LoadLibraryA(c_str!("user32.dll"));
            let shcore = LoadLibraryA(c_str!("shcore.dll"));

            DpiFns {
                SetProcessDPIAware: load!(user32, "SetProcessDPIAware"),
                SetProcessDpiAwareness: load!(shcore, "SetProcessDpiAwareness"),
                SetProcessDpiAwarenessContext: load!(user32, "SetProcessDpiAwarenessContext"),
                GetDpiForMonitor: load!(shcore, "GetDpiForMonitor"),
                GetDpiForWindow: load!(user32, "GetDpiForWindow"),
                EnableNonClientDpiScaling: load!(user32, "EnableNonClientDpiScaling"),
                AdjustWindowRectExForDpi: load!(user32, "AdjustWindowRectExForDpi"),
            }
        }
    }
}
