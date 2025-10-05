use crate::Result;
use crate::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFIndex, CFTypeRef, Error};

pub struct Array<T: TryFrom<CFTypeRef, Error = Error>>(Vec<T>);

impl<T> TryFrom<CFArrayRef> for Array<T>
where
    T: TryFrom<CFTypeRef, Error = Error>,
{
    type Error = Error;
    fn try_from(array: CFArrayRef) -> Result<Array<T>> {
        if array.is_null() {
            // TODO: real error
            return Err(Error::NulString);
        }

        let len = unsafe { CFArrayGetCount(array) };

        let vec = (0..len)
            .map(|i| {
                let type_ref = unsafe { CFArrayGetValueAtIndex(array, i as CFIndex) };
                T::try_from(CFTypeRef(type_ref))
            })
            .collect::<Result<Vec<T>>>()?;

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
