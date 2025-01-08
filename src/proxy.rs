use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::{InstBuilder as _, MemFlags};
use cranelift::prelude::*;

use crate::func::{with_ctx, FnCtx, Param, };
use crate::primitive::ToPrimitive;
use crate::val::{AsVal, Val};

impl<T: ToPrimitive> Ref<T> {
    fn load(&self, ctx: &mut FnCtx) -> Val<T> {
        Val::from_value(ctx.builder.ins().load(T::ty(), MemFlags::new(), self.addr.value(), self.offset))
    }

    pub fn get(&self) -> Val<T> {
        with_ctx(|ctx| {
            self.load(ctx)
        })
    }
}

#[repr(transparent)]
pub struct RefMut<T>(Ref<T>);

impl<T> RefMut<T> {
    #[doc(hidden)]
    pub fn new(addr: Val<usize>, offset: i32) -> Self {
        Self(Ref::new(addr, offset))
    }
}

impl<T> Deref for RefMut<T> {
    type Target = Ref<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> RefMut<T> {
    fn store(&mut self, ctx: &mut FnCtx, val: Value) {
        ctx.builder.ins().store(MemFlags::new(), val, self.addr.value(), self.offset);
    }

    pub fn put(&mut self, val: impl AsVal<Ty = T>) {
        with_ctx(|ctx| {
            let val = val.as_val(ctx);
            self.store(ctx, val.value());
        });
    }
}

#[derive(Clone, Copy)]
pub struct Ref<T> {
    addr: Val<usize>,
    offset: i32,
    _pth: PhantomData<T>,
}

impl<T: ToPrimitive> AsVal for Ref<T> {
    type Ty = T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        self.load(ctx)
    }
}

impl<T> Ref<T> {
    #[doc(hidden)]
    pub fn addr(&self) -> Val<usize> {
        self.addr
    }

    #[doc(hidden)]
    pub fn offset(&self) -> i32 {
        self.offset
    }

    #[doc(hidden)]
    pub fn new(addr: Val<usize>, offset: i32) -> Self {
        Self { addr, offset, _pth: PhantomData }
    }
}

impl<'a, T> AsVal for &'a Ref<T> {
    type Ty = &'a T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        let addr = if self.offset() != 0 {
            ctx.builder().ins().iadd_imm(self.addr().value(), self.offset() as i64)
        } else {
            self.addr().value()
        };
        Val::from_value(addr)
    }
}

impl<'a, T> AsVal for &'a mut RefMut<T> {
    type Ty = &'a mut T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        let p = &self.0;
        Val::from_value(p.as_val(ctx).value())
    }
}

pub struct Slice<T> {
    base: Val<usize>,
    len: Val<usize>,
    _t: PhantomData<T>,
}

impl<T> Slice<T> {
    pub fn len(&self) -> Val<usize> {
        self.len
    }

    pub  fn get(&self, idx: impl AsVal<Ty = usize>) -> Ref<T> {
        // TODO bound checks?
        let offset = self.base + idx.value() * size_of::<T>();
        Ref::new(offset, 0)
    }
}

impl<T> Param for &[T] {
    type Ty = Slice<T>;

    fn initialize_param_at(ctx: &mut FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        // this is UB, since the can't make assumption about the representation of &[T]
        let len = usize::initialize_param_at(ctx, idxs);
        let base = usize::initialize_param_at(ctx, idxs);
        Slice {
            base: base.as_val(ctx),
            len: len.as_val(ctx),
            _t: PhantomData,
        }
    }
}
