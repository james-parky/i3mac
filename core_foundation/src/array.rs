use crate::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, CFIndex, CFTypeRef, Error, Result,
};

#[derive(Debug)]
pub struct Array<T: TryFrom<CFTypeRef, Error = Error>>(Vec<T>);

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

impl<T> Array<T>
where
    T: TryFrom<CFTypeRef, Error = Error>,
{
    pub unsafe fn try_from_raw(array: CFArrayRef) -> Result<Array<T>> {
        if array.is_null() {
            return Err(Error::NullCFArray);
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
