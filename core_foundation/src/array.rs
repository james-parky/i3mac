use crate::Result;
use crate::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFIndex, CFTypeRef, Error};

pub struct Array(Vec<CFTypeRef>);

impl Array {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn try_create(array_ref: CFArrayRef) -> Result<Array> {
        if array_ref.is_null() {
            // TODO: real error
            return Err(Error::NulString);
        }
        let len = unsafe { CFArrayGetCount(array_ref) };
        let mut vec = Vec::with_capacity(len as usize);

        for i in 0..len as usize {
            let type_ref = unsafe { CFArrayGetValueAtIndex(array_ref, i as CFIndex) };
            vec.push(CFTypeRef(type_ref));
        }

        Ok(Array(vec))
    }

    pub fn get<T>(&self, index: usize) -> Result<T>
    where
        T: TryFrom<CFTypeRef, Error = Error>,
    {
        if index >= self.0.len() {
            // TODO: error
            Err(Error::NulString)
        } else {
            let value = self.0[index];
            T::try_from(value)
        }
    }
}
