use std::{ptr, slice};

use cocoa::base::id;
use cocoa::quartzcore::{CALayer, ContentsGravity, Filter};

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

use super::ffi::*;

const BYTES_PER_ELEMENT: usize = 4;

pub struct Surface {
    pub layer: CALayer,
    pub surface: IOSurfaceRef,
    pub width: usize,
    pub height: usize,
}

impl Surface {
    pub fn new(width: usize, height: usize) -> Surface {
        unsafe {
            let properties = CFDictionary::from_CFType_pairs(&[
                (
                    CFString::wrap_under_get_rule(kIOSurfaceWidth),
                    CFNumber::from(width as i32).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kIOSurfaceHeight),
                    CFNumber::from(height as i32).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kIOSurfaceBytesPerElement),
                    CFNumber::from(BYTES_PER_ELEMENT as i32).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kIOSurfacePixelFormat),
                    CFNumber::from(kCVPixelFormatType_32BGRA).as_CFType(),
                ),
            ]);

            let surface = IOSurfaceCreate(properties.as_concrete_TypeRef());

            IOSurfaceSetValue(
                surface,
                kIOSurfaceColorSpace,
                kCGColorSpaceSRGB as CFTypeRef,
            );

            let layer = CALayer::new();
            layer.set_contents(surface as id);
            layer.set_opaque(true);
            layer.set_contents_opaque(true);
            layer.set_contents_gravity(ContentsGravity::TopLeft);
            layer.set_magnification_filter(Filter::Nearest);

            Surface {
                layer,
                surface,
                width,
                height,
            }
        }
    }

    pub fn with_buffer<F: FnOnce(&mut [u32])>(&mut self, f: F) {
        unsafe {
            if IOSurfaceLock(self.surface, 0, ptr::null_mut()) != kIOSurfaceSuccess {
                return;
            }

            let addr = IOSurfaceGetBaseAddress(self.surface);
            let buffer = slice::from_raw_parts_mut(addr as *mut u32, self.width * self.height);
            f(buffer);

            IOSurfaceUnlock(self.surface, 0, ptr::null_mut());
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.surface as CFTypeRef);
        }
    }
}
