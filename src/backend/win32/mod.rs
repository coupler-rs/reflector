use std::ffi::OsStr;
use std::fmt;
use std::os::windows::ffi::OsStrExt;

use winapi::{shared::minwindef, shared::ntdef, um::winnt};

mod app;
mod timer;
mod window;

pub use app::{AppContextInner, AppInner};
pub use timer::TimerHandleInner;
pub use window::WindowInner;

fn hinstance() -> minwindef::HINSTANCE {
    extern "C" {
        static __ImageBase: winnt::IMAGE_DOS_HEADER;
    }

    unsafe { &__ImageBase as *const winnt::IMAGE_DOS_HEADER as minwindef::HINSTANCE }
}

fn to_wstring<S: AsRef<OsStr> + ?Sized>(str: &S) -> Vec<ntdef::WCHAR> {
    let mut wstr: Vec<ntdef::WCHAR> = str.as_ref().encode_wide().collect();
    wstr.push(0);
    wstr
}

#[derive(Debug)]
pub struct OsError {
    code: minwindef::DWORD,
}

impl fmt::Display for OsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.code)
    }
}
