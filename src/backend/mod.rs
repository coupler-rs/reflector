#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "windows")]
pub use win32::*;

#[cfg(target_os = "macos")]
mod cocoa;
#[cfg(target_os = "macos")]
pub use cocoa::*;

#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "linux")]
pub use x11::*;
