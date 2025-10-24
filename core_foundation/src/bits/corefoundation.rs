//! Core Foundation
//!
//! Access low-level functions, primitive data types, and various collection
//! types that are bridged seamlessly with the Foundation framework.
//!
//! # Overview
//!
//! Core Foundation is a framework that provides fundamental software services
//! useful to application services, application environments, and to
//! applications themselves. Core Foundation also provides abstractions for
//! common data types, facilitates internationalisation with Unicode string
//! storage, and offers a suite of utilities such as plug-in support, XML
//! property lists, URL resource access, and preferences.

use crate::Error;
use crate::Result;

use std::ffi::{CStr, CString, c_char, c_ulong, c_void};

pub type CFArrayRef = *const CFArray;
pub type CFArray = c_void;

pub type CFRunLoopSourceRef = *const c_void;
pub type CFRunLoopRef = *const c_void;
pub type CFRunLoopMode = CFStringRef;

pub type CFIndex = c_ulong;
pub type CFDictionaryRef = *const CFDictionary;
pub type CFDictionary = c_void;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct CFTypeRef(pub *const c_void);

pub type CFBooleanRef = *const c_void;

type CFAllocatorRef = *const c_void;

/// `CFString` provides a suite of efficient string-manipulation and
/// string-conversion functions. It offers seamless Unicode support and
/// facilitates the sharing of data between Cocoa and C-based programs.
/// `CFString` objects are immutable—use `CFMutableStringRef` to create and
/// manage a string that can be changed after it has been created.
///
/// `CFString` has two primitive functions, `CFStringGetLength` and
/// `CFStringGetCharacterAtIndex`, that provide the basis for all other
/// functions in its interface. The `CFStringGetLength` function returns the
/// total number (in terms of UTF-16 code pairs) of characters in the string.
/// The `CFStringGetCharacterAtIndex` function gives access to each character
/// in the string by index, with index values starting at 0.
///
/// `CFString` provides functions for finding and comparing strings. It also
/// provides functions for reading numeric values from strings, for combining
/// strings in various ways, and for converting a string to different forms
/// (such as encoding and case changes). A number of functions, for example
/// `CFStringFindWithOptions`, allow you to specify a range over which to
/// operate within a string. The specified range must not exceed the length of
/// the string. Debugging options may help you to catch any errors that arise if
/// a range does exceed a string’s length.
///
/// Like other `core_foundation` types, you can hash `CFString`s using the
/// `CFHash` function. You should never, though, store a hash value outside of
/// your application and expect it to be useful if you read it back in later
/// (hash values may change between different releases of the operating system).
///
/// `CFString` is “toll-free bridged” with its Cocoa Foundation counterpart,
/// `NSString`. This means that the `core_foundation` type is interchangeable in
/// function or method calls with the bridged Foundation object. Therefore, in a
/// method where you see an `NSString*` parameter, you can pass in a
/// `CFStringRef`, and in a function where you see a `CFStringRef` parameter,
/// you can pass in an `NSString` instance. This also applies to concrete
/// subclasses of `NSString`. See Toll-Free Bridged Types for more information
/// on toll-free bridging.
pub type CFStringRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Returns the number of values currently in an array.
    ///
    /// # Arguments
    ///
    /// * `the_array` - The array to examine.
    ///
    /// # Returns
    ///
    /// The number of values in the array.
    pub fn CFArrayGetCount(the_array: CFArrayRef) -> CFIndex;

    /// Retrieves a value at a given index.
    ///
    /// # Arguments
    ///
    /// * `the_array` - The array to examine.
    /// * `idx` - The index of the value to retrieve. If the index is outside
    ///   the index space of `the_array` (0 to N-1 inclusive (where N is the
    ///   count of `the_array`)), the behaviour is undefined.
    ///
    /// # Returns
    ///
    /// The value at the `idx` in `the_array`. If the return value is a
    /// `core_foundation` object, ownership follows 'The Get Rule'.
    pub fn CFArrayGetValueAtIndex(the_array: CFArrayRef, idx: CFIndex) -> *const c_void;

    /// Returns the number of key-value pairs in a dictionary.
    ///
    /// # Arguments
    ///
    /// * `the_dict` - The dictionary to examine.
    ///
    /// # Returns
    ///
    /// The number of key-value pairs in `the_dict`.
    pub fn CFDictionaryGetCount(the_dict: CFDictionaryRef) -> CFIndex;

    /// Fills two buffers with the keys and values from a dictionary.
    ///
    /// # Arguments
    ///
    /// * `the_dict` - The dictionary to examine.
    /// * `keys` - A C array of pointer-sized values that, on return, is filled
    ///   with keys from `the_dict`. The keys and values C arrays are parallel
    ///   to each other (that is, the items are the same indices form a
    ///   key-value pair from the dictionary). This value must be a valid
    ///   pointer to a C array of the appropriate type and size (that is, a size
    ///   equal to the count of `the_dict`), or `NULL` if the keys are not
    ///   required. If the keys are `core_foundation` objects, ownership follows
    ///   'The Get Rule'.
    /// * `values` - A C array of pointer-sized values that, on return, is
    ///   filled with values from `the_dict`. The keys and values C arrays are
    ///   parallel to each other (that is, the items at the same indices form a
    ///   key-value pair from the dictionary). This value must be a valid
    ///   pointer to a C array of the appropriate type and size (that is, a size
    ///   equal to the count of `the_dict`), or `NULL` if the values are not
    ///   required. If the values are `core_foundation` objects, ownership
    ///   follows 'The Get Rule'.
    pub fn CFDictionaryGetKeysAndValues(
        the_dict: CFDictionaryRef,
        keys: *mut *const c_void,
        values: *mut *const c_void,
    );

    /// Creates an immutable string from a C string.
    ///
    /// # Arguments
    ///
    /// * `alloc` - The allocator to use to allocate memory for the new string.
    ///   Pass `NULL` or `kCFAllocatorDefault` to use the current default
    ///   allocator.
    /// * `c_str` - The `NULL`-terminated C string to be used to create the
    ///   `CFString` object. The string must use an 8-bit encoding.
    /// * `encoding` - The encoding of the characters in the C string. The
    ///   encoding must specify an 8-bit encoding.
    ///
    /// # Returns
    ///
    /// An immutable string containing `c_str` (after stripping off the `NULL`
    /// terminating character), or `NULL` if there was a problem creating the
    /// object. Ownership follows 'The Create Rule'.
    ///
    /// # Discussion
    ///
    /// A C string is a string of 8-bit characters terminated with an 8-bit
    /// `NULL`. Unichar and Unichar32 are not considered C strings.
    pub fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const c_char,
        encoding: CFStringEncoding,
    ) -> CFStringRef;

    /// Copies the character contents of a string to a local C string buffer
    /// after converting the characters to a given encoding.
    ///
    /// # Arguments
    ///
    /// * `the_string` - The string whose contents you wish to access.
    /// * `buffer` - The C string buffer into which to copy the string. On
    ///   return, the buffer contains the converted characters. If there is an
    ///   error in conversion, the buffer contains only partial results.
    /// * `buffer_size` - The length of `buffer` in bytes.
    /// * `encoding` - The string encoding to which the character contents of
    ///   `the_string` should be converted. The encoding must specify an 8-bit
    ///   encoding.
    ///
    /// # Returns
    ///
    /// `true` upon success or `false` if the conversion fails or the provided
    /// buffer is too small.
    ///
    /// # Discussion
    ///
    /// This function is useful when you need your own copy of a string's
    /// character data as a C string. You also typically call it as a "backup"
    /// when a prior call to the `CFStringGetCStringPtr` function fails.
    pub fn CFStringGetCString(
        the_string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> bool;

    /// Returns the number (in terms of UTF-16 code pairs) of Unicode characters
    /// in a string.
    ///
    /// # Arguments
    ///
    /// * `the_string` - The string to examine.
    ///
    /// # Returns
    ///
    /// The number (in terms of UTF-16 code pairs) of characters stored in
    /// `the_string`.
    pub fn CFStringGetLength(the_string: CFStringRef) -> CFIndex;

    /// Returns the maximum number of bytes a string of a specified length (in
    /// Unicode characters) will take up if encoded in a specific encoding.
    ///
    /// # Arguments
    ///
    /// * `length` - The number of Unicode characters to evaluate.
    /// * `encoding` - The string encoding for the number of characters
    ///   specified by `length`.
    ///
    /// # Returns
    ///
    /// The maximum number of bytes that could be need to represent `length`
    /// number of Unicode characters with the string encoding `encoding`, or
    /// `kCFNotFound` if the number exceeds `LONG_MAX`.
    ///
    /// # Discussion
    ///
    /// The number of bytes that the encoding actually ends up requiring when
    /// converting any particular string could be less than the returned value,
    /// but never more.
    pub fn CFStringGetMaximumSizeForEncoding(index: CFIndex, encoding: CFStringEncoding)
    -> CFIndex;

    /// Quickly obtains a pointer to a C-string buffer containing the characters
    /// of a string in a given encoding.
    ///
    /// # Arguments
    ///
    /// * `the_string` - The string whose contents you wish to access.
    /// * `encoding` - The string encoding to which the character contents of
    ///   `the_string` should be converted. The encoding must specify and 8-bit
    ///   encoding.
    ///
    /// # Returns
    ///
    /// A pointer to a C string or `NULL` if the internal storage of
    /// `the_string` does not allow this to be returned efficiently.
    ///
    /// # Discussion
    ///
    /// This function either returns the requested pointer immediately, with no
    /// memory allocations and no copying, in constant time, or returns `NULL`.
    /// If the latter is the result, call an alternative function such as the
    /// `CFStringGetCString` function to extract the characters.
    ///
    /// Whether or not this function returns a valid pointer or `NULL` depends
    /// on many factors, all of which depend on how the string was created and
    /// its properties. In addition, the function result might change between
    /// different releases and on different platforms. So do not count on
    /// receiving a non-`NULL` result from this function under any
    /// circumstances.
    pub fn CFStringGetCStringPtr(
        the_string: CFStringRef,
        encoding: CFStringEncoding,
    ) -> *const c_char;

    /// Obtains the value of a `CFNumber` object cast to a specific type.
    ///
    /// # Arguments
    ///
    /// * `number` - The `CFNumber` object to examine.
    /// * `the_type` - A constant that specifies the data type to return.
    /// * `value_ptr` - On return, contains the value of `number`.
    ///
    /// # Returns
    ///
    /// `true` if the operation was successful, otherwise `false`.
    ///
    /// # Discussion
    ///
    /// If the argument `ype differs from the return type, and the conversion is
    /// lossy or the return value is out of range, then its function passes
    /// back and approximate value in `value_ptr` and returns `false`.
    fn CFNumberGetValue(
        number: CFNumberRef,
        the_type: CFNumberType,
        value_ptr: *mut c_void,
    ) -> bool;

    static kCFBooleanTrue: CFBooleanRef;
    static kCFBooleanFalse: CFBooleanRef;

    static kCFAllocatorDefault: CFAllocatorRef;

    /// Determines whether two `core_foundation` objects are considered equal.
    ///
    /// # Arguments
    ///
    /// * `cf1` - A `CFType` object to compare to `cf2`.
    /// * `cf2` - A `CFType` object to compare to `cf1`.
    ///
    /// # Returns
    ///
    /// `true` if `cf1` and `cf2` are of the same type and considered equal,
    /// otherwise `false`.
    ///
    /// # Discussion
    ///
    /// Equality is something specific to each `core_foundation` opaque type.
    /// For example, two `CFNumber` objects are equal if the numeric values they
    /// represent are equal. Two `CFString` objects are equal if they represent
    /// identical sequences of characters, regardless of encoding.
    pub fn CFEqual(cf1: CFTypeRef, cf2: CFTypeRef) -> bool;

    /// Returns a code that can be used to identify an object in a hashing
    /// structure.
    ///
    /// # Arguments
    ///
    /// * `cf` - A `CFType` object to examine.
    ///
    /// # Returns
    ///
    /// An integer of type `CFHashCode` that represents a hashing value for
    /// `cf`.
    ///
    /// # Discussion
    ///
    /// Two objects that are equal (as determined by the `CFEqual` function)
    /// have the same hashing value. However, the converse is not true; two
    /// objects with the same hashing value might not be equal. That is, hashing
    /// values are not necessarily unique.
    ///
    /// The hashing value for an object might change from release to release or
    /// from platform to platform.
    pub fn CFHash(cf: CFTypeRef) -> CFHashCode;

    fn CFBooleanGetValue(boolean: CFBooleanRef) -> bool;

    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFRunLoopMode);
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopRun();
    pub static kCFRunLoopDefaultMode: CFRunLoopMode;
}

type CFHashCode = usize;

#[repr(C)]
// We use an enum here to be faithful to the Core Graphics library signatures,
// but we only ever need the Utf8 variant.
#[allow(dead_code)]
pub enum CFStringEncoding {
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
