use crate::val::Val;
use crate::proxy::{Proxy, Slice};
use crate::func::{IntoHostFn as _, Param};

impl<T: Param> Proxy<Vec<T>> {
    pub fn new() -> Self {
        Self::ctor(Vec::new)
    }

    pub fn push(&mut self, val: Val<T>) {
        let f = (|this: &mut Vec<T>, t: T| {
            this.push(t);
        }).into_host_fn();

        f.call((self.get_mut(), val))
    }

    pub fn len(&self) -> Val<usize> {
        let f = (|this: &Vec<T>| -> usize {
            this.len()
        }).into_host_fn();

        f.call(self.get_ref())
    }

    fn as_ptr(&self) -> Val<*const T> {
        let f = (|this: &Vec<T>| -> *const T {
            this.as_ptr()
        }).into_host_fn();

        f.call(self.get_ref())
    }

    pub fn as_slice(&mut self) -> Slice<T> {
        Slice {
            base: self.as_ptr(),
            len: self.len(),
            _p: std::marker::PhantomData,
        }
    }
}
