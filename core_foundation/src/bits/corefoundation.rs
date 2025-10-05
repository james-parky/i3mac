use crate::Error;
use crate::Result;

use std::ffi::{CStr, CString, c_char, c_ulong, c_void};

pub type CFArrayRef = *const CFArray;
pub type CFArray = c_void;

pub type CFIndex = c_ulong;
pub type CFDictionaryRef = *const CFDictionary;
pub type CFDictionary = c_void;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct CFTypeRef(pub *const c_void);

pub type CFBooleanRef = *const c_void;

type CFAllocatorRef = *const c_void;
type CFStringRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> *const c_void;
    pub fn CFDictionaryGetCount(dictionary: CFDictionaryRef) -> CFIndex;
    pub fn CFDictionaryGetKeysAndValues(
        dictionary: CFDictionaryRef,
        keys: *mut *const c_void,
        values: *mut *const c_void,
    ) -> *const c_void;
    fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        string: *const c_char,
        encoding: CFStringEncoding,
    ) -> CFStringRef;
    fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> bool;
    fn CFStringGetLength(string: CFStringRef) -> CFIndex;
    fn CFStringGetMaximumSizeForEncoding(index: CFIndex, encoding: CFStringEncoding) -> CFIndex;

    fn CFNumberGetValue(number: CFNumberRef, type_: CFNumberType, value: *mut c_void) -> bool;

    static kCFBooleanTrue: CFBooleanRef;
    static kCFBooleanFalse: CFBooleanRef;

    static kCFAllocatorDefault: CFAllocatorRef;

    pub fn CFEqual(a: CFTypeRef, b: CFTypeRef) -> bool;
    pub fn CFHash(hash: CFTypeRef) -> usize;

    fn CFBooleanGetValue(boolean: CFBooleanRef) -> bool;
}

#[repr(C)]
// We use an enum here to be faithful to the Core Graphics library signatures,
// but we only ever need the Utf8 variant.
#[allow(dead_code)]
enum CFStringEncoding {
    MacRoman = 0,
    WindowsLatin1 = 0x0500,
    IsoLatin1 = 0x0201,
    NextStepLatin = 0x0B01,
    Ascii = 0x0600,
    Unicode = 0x0100,
    Utf8 = 0x0800_0100,
    NonLossyAscii = 0x0BFF,
}

#[repr(transparent)]
// We use a new-type wrapper around the CFIndex and implement associated
// constants here rather than an enum to be faithful to the Core Graphics
// library signatures, but we only ever need a few of them.
pub struct CFNumberType(CFIndex);
impl CFNumberType {
    #[allow(dead_code)]
    pub const INT8: Self = Self(1);
    #[allow(dead_code)]
    pub const INT16: Self = Self(2);
    pub const INT32: Self = Self(3);
    #[allow(dead_code)]
    pub const INT64: Self = Self(4);
    pub const FLOAT32: Self = Self(5);
    #[allow(dead_code)]
    pub const FLOAT64: Self = Self(6);
    #[allow(dead_code)]
    pub const CHAR: Self = Self(7);
    #[allow(dead_code)]
    pub const SHORT: Self = Self(8);
    #[allow(dead_code)]
    pub const INT: Self = Self(9);
    #[allow(dead_code)]
    pub const LONG: Self = Self(10);
    pub const LONG_LONG: Self = Self(11);
    #[allow(dead_code)]
    pub const FLOAT: Self = Self(12);
    pub const DOUBLE: Self = Self(13);
    #[allow(dead_code)]
    pub const CF_INDEX: Self = Self(14);
    #[allow(dead_code)]
    pub const NS_INTEGER: Self = Self(15);
    #[allow(dead_code)]
    pub const CF_FLOAT: Self = Self(16);
}

impl TryFrom<bool> for CFTypeRef {
    type Error = Error;

    fn try_from(value: bool) -> std::result::Result<Self, Self::Error> {
        Ok(if value {
            CFTypeRef(unsafe { kCFBooleanTrue })
        } else {
            CFTypeRef(unsafe { kCFBooleanFalse })
        })
    }
}

impl TryFrom<&str> for CFTypeRef {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let c_string = CString::new(value).map_err(Error::CannotCreateCString)?;

        let string_ref = unsafe {
            CFStringCreateWithCString(
                kCFAllocatorDefault,
                c_string.as_ptr(),
                CFStringEncoding::Utf8,
            )
        };

        if string_ref.is_null() {
            Err(Error::NulString)
        } else {
            Ok(CFTypeRef(string_ref))
        }
    }
}

impl TryFrom<CFTypeRef> for &str {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> Result<Self> {
        if value.0.is_null() {
            return Err(Error::NulString);
        }

        let len: CFIndex = unsafe { CFStringGetLength(value.0 as CFStringRef) };
        let max_size = unsafe { CFStringGetMaximumSizeForEncoding(len, CFStringEncoding::Utf8) };

        let mut buffer = vec![0u8; max_size as usize];
        let success = unsafe {
            CFStringGetCString(
                value.0 as CFStringRef,
                buffer.as_mut_ptr().cast(),
                max_size,
                CFStringEncoding::Utf8,
            )
        };

        if success {
            let cstr = unsafe { CStr::from_ptr(buffer.as_ptr().cast()) };
            cstr.to_str().map_err(Error::InvalidCString)
        } else {
            // TODO: specific error type
            Err(Error::NulString)
        }
    }
}

impl TryFrom<String> for CFTypeRef {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let c_string = CString::new(value).map_err(Error::CannotCreateCString)?;

        let string_ref = unsafe {
            CFStringCreateWithCString(
                kCFAllocatorDefault,
                c_string.as_ptr(),
                CFStringEncoding::Utf8,
            )
        };

        if string_ref.is_null() {
            Err(Error::NulString)
        } else {
            Ok(CFTypeRef(string_ref))
        }
    }
}

impl TryFrom<CFTypeRef> for String {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        if value.0.is_null() {
            return Err(Error::NulString);
        }

        let len: CFIndex = unsafe { CFStringGetLength(value.0 as CFStringRef) };
        let max_size = unsafe { CFStringGetMaximumSizeForEncoding(len, CFStringEncoding::Utf8) };

        let mut buffer = vec![0u8; max_size as usize];
        let success = unsafe {
            CFStringGetCString(
                value.0 as CFStringRef,
                buffer.as_mut_ptr().cast(),
                max_size,
                CFStringEncoding::Utf8,
            )
        };

        if success {
            let cstr = unsafe { CStr::from_ptr(buffer.as_ptr().cast()) };
            cstr.to_str()
                .map(String::from)
                .map_err(Error::InvalidCString)
        } else {
            // TODO: specific error type
            Err(Error::NulString)
        }
    }
}

type CFNumberRef = *const c_void;

pub fn cf_type_ref_to_num<T: Default>(cf: CFTypeRef, type_: CFNumberType) -> Result<T> {
    if cf.0.is_null() {
        // TODO: different error type
        return Err(Error::NulString);
    }

    let mut out = T::default();
    let ok = unsafe {
        CFNumberGetValue(
            cf.0 as CFNumberRef,
            type_, // TODO: needs constants or enum values
            (&raw mut out).cast(),
        )
    };

    if ok {
        Ok(out)
    } else {
        // TODO: different error type
        Err(Error::NulString)
    }
}

impl TryFrom<CFTypeRef> for i32 {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        cf_type_ref_to_num(value, CFNumberType::INT32)
    }
}

impl TryFrom<CFTypeRef> for u64 {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        cf_type_ref_to_num(value, CFNumberType::LONG_LONG)
    }
}

impl TryFrom<CFTypeRef> for f32 {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        cf_type_ref_to_num(value, CFNumberType::FLOAT32)
    }
}

impl TryFrom<CFTypeRef> for f64 {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        cf_type_ref_to_num(value, CFNumberType::DOUBLE)
    }
}

impl TryFrom<CFTypeRef> for bool {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        if value.0.is_null() {
            // TODO: error enum
            return Err(Error::NulString);
        }

        Ok(unsafe { CFBooleanGetValue(value.0 as CFBooleanRef) })
    }
}

impl TryFrom<CFTypeRef> for CFDictionaryRef {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> std::result::Result<Self, Self::Error> {
        Ok(value.0 as CFDictionaryRef)
    }
}
