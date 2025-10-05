use crate::Result;
use crate::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFIndex, CFTypeRef, Error};

pub struct Array<T: TryFrom<CFTypeRef, Error = Error>>(Vec<T>);

impl<T> Array<T>
where
    T: TryFrom<CFTypeRef, Error = Error>,
{
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn try_create(array_ref: CFArrayRef) -> Result<Array<T>> {
        if array_ref.is_null() {
            // TODO: real error
            return Err(Error::NulString);
        }

        let len = unsafe { CFArrayGetCount(array_ref) };
        let mut vec = Vec::with_capacity(len as usize);

        for i in 0..len as usize {
            let type_ref = unsafe { CFArrayGetValueAtIndex(array_ref, i as CFIndex) };
            vec.push(T::try_from(CFTypeRef(type_ref))?);
        }

        Ok(Array(vec))
    }
}

impl<T> IntoIterator for Array<T>
where
    T: TryFrom<CFTypeRef, Error = Error>,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
