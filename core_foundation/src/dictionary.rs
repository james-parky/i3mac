use crate::{
    CFRelease, CFRetain, Error, Result,
    bits::{
        CFDictionaryGetCount, CFDictionaryGetKeysAndValues, CFDictionaryRef, CFEqual, CFHash,
        CFTypeRef,
    },
};
use std::{collections::HashMap, ffi::c_void, hash::Hash};

#[derive(Debug)]
struct CFKey(CFTypeRef);

impl PartialEq for CFKey {
    fn eq(&self, other: &Self) -> bool {
        unsafe { CFEqual(self.0, other.0) }
    }
}

impl Eq for CFKey {}

impl Hash for CFKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let h = unsafe { CFHash(self.0) };
        h.hash(state);
    }
}

#[derive(Debug)]
pub struct Dictionary(HashMap<CFKey, CFTypeRef>);

impl TryFrom<CFTypeRef> for Dictionary {
    type Error = Error;
    fn try_from(value: CFTypeRef) -> Result<Self> {
        unsafe { Dictionary::try_from_raw(value.0 as CFDictionaryRef) }
    }
}

impl Dictionary {
    pub fn get<K, V>(&self, key: &K) -> Option<V>
    where
        K: TryInto<CFTypeRef> + Copy,
        V: TryFrom<CFTypeRef>,
    {
        let cf_key: CFTypeRef = (*key).try_into().ok()?;
        let value = self.0.get(&CFKey(cf_key));
        unsafe { CFRelease(cf_key) };
        value.and_then(|&v| V::try_from(v).ok())
    }

    pub unsafe fn try_from_raw(dict: CFDictionaryRef) -> Result<Self> {
        if dict.is_null() {
            return Err(Error::NulDictionary);
        }

        let size: u64 = unsafe { CFDictionaryGetCount(dict) };
        let mut keys: Vec<*const c_void> = vec![std::ptr::null(); size as usize];
        let mut values: Vec<*const c_void> = vec![std::ptr::null(); size as usize];
        let mut inner = HashMap::with_capacity(size as usize);

        unsafe { CFDictionaryGetKeysAndValues(dict, keys.as_mut_ptr(), values.as_mut_ptr()) };

        for i in 0..size as usize {
            let key_ref = CFTypeRef(keys[i]);
            let value_ref = CFTypeRef(values[i]);

            unsafe { CFRetain(key_ref) };
            unsafe { CFRetain(value_ref) };
            inner.insert(CFKey(key_ref), value_ref);
        }

        Ok(Dictionary(inner))
    }
}

impl Drop for Dictionary {
    fn drop(&mut self) {
        unsafe {
            for (key, value) in &self.0 {
                CFRelease(key.0);
                CFRelease(*value);
            }
        }
    }
}
