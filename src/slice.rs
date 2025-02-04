use std::marker::PhantomData;

use crate::cmp::Compare;
use crate::func::{FnCtx, Param};
use crate::iterator::JIterator;
use crate::prelude::IntoJiter;
use crate::proxy::Ref;
use crate::val::{AsVal, Val};
use crate::var::Var;

pub struct Slice<'a, T> {
    pub base: Val<*const T>,
    pub len: Val<usize>,
    pub _p: PhantomData<&'a [T]>,
}

impl<T> Clone for Slice<'_, T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Copy for Slice<'_, T> { }

impl<'a, T> Slice<'a, T> {
    pub fn len(&self) -> Val<usize> {
        self.len
    }

    pub fn get(&self, idx: impl AsVal<Ty = usize>) -> Ref<'a, T> {
        // TODO bound checks?
        let offset = Val::<usize>::from(self.base) + idx.value() * size_of::<T>();
        let ptr = unsafe { offset.transmute() };
        Ref::new(ptr)
    }
}

impl<'a, T> IntoJiter for Slice<'a, T> {
    type Iter = SliceIter<'a, T>;
    type Item = Ref<'a, T>;

    fn into_jiter(self) -> Self::Iter {
        SliceIter {
            index: Var::new(0usize),
            slice: self,
        }
    }
}

impl<'a, T> Param for &'a [T] {
    type Ty = Slice<'a, T>;

    fn initialize_param_at(ctx: &mut FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        let len = usize::initialize_param_at(ctx, idxs);
        let base = <*const T>::initialize_param_at(ctx, idxs);
        Slice {
            base: base.addr.as_val(ctx),
            len: len.as_val(ctx),
            _p: PhantomData,
        }
    }
}

pub struct SliceIter<'a, T> {
    index: Var<usize>,
    slice: Slice<'a, T>,
}

impl<'a, T> JIterator for SliceIter<'a, T> {
    type Item = Ref<'a, T>;

    fn next(&mut self) -> (Val<bool>, impl FnOnce() -> Self::Item) {
        let s = self.slice;
        let index = self.index;
        let ret = (self.index.value().neq(self.slice.len()), move || {
            let val = s.get(index);
            self.index += 1usize;
            val
        });
        ret
    }
}
