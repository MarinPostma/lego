use std::marker::PhantomData;

use cranelift::prelude::InstBuilder as _;
use cranelift::prelude::Value;

use crate::func::with_ctx;
use crate::func::FnCtx;
use crate::primitive::Primitive;
use crate::proxy::PtrMut;

pub struct Val<T> {
    value: Value,
    _pth: PhantomData<T>,
}

impl<T> Copy for Val<T> {}
impl<T> Clone for Val<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Val<T> {
    pub fn new(val: T) -> Val<T>
    where
        T: Primitive,
    {
        with_ctx(|ctx| {
            let val = ctx.builder().ins().iconst(T::ty(), val.to_i64());
            Val::from_value(val)
        })
    }

    // TODO: this should be unsafe
    pub(crate) fn from_value(value: Value) -> Self {
        Self {
            value,
            _pth: PhantomData,
        }
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }

    pub(crate) unsafe fn transmute<U>(self) -> Val<U> {
        Val::from_value(self.value())
    }
}

//TODO: macros for safe casts
impl<T> From<Val<*mut T>> for Val<*const T> {
    fn from(value: Val<*mut T>) -> Self {
        // it's always safe to get a *const T from a *mut T, and they are the safe size
        unsafe { value.transmute() }
    }
}

impl<T> From<Val<*const T>> for Val<usize> {
    fn from(value: Val<*const T>) -> Self {
        unsafe {
            // this is safe, since usize is pointer sized
            value.transmute()
        }
    }
}

impl<T> From<Val<*mut T>> for Val<usize> {
    fn from(value: Val<*mut T>) -> Self {
        unsafe {
            // this is safe, since usize is pointer sized
            value.transmute()
        }
    }
}

pub trait AsVal {
    type Ty;

    fn value(&self) -> Val<Self::Ty> {
        with_ctx(|ctx| self.as_val(ctx))
    }

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty>;
}

macro_rules! impl_into_var_primitive {
    ($($prim:ident $(,)?)*) => {
        $(
            impl AsVal for $prim {
                type Ty = $prim;
                fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
                    let value = ctx.builder.ins().iconst(Self::ty(), *self as i64);
                    Val::from_value(value)
                }
            }
        )*
    };
}

impl<A: AsVal> AsVal for (A,) {
    type Ty = A::Ty;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        self.0.as_val(ctx)
    }
}

impl<T> AsVal for &mut PtrMut<T> {
    type Ty = *mut T;

    fn as_val(&self, _ctx: &mut FnCtx) -> Val<Self::Ty> {
        self.addr
    }
}

impl<T> AsVal for &PtrMut<T> {
    type Ty = *const T;

    fn as_val(&self, _ctx: &mut FnCtx) -> Val<Self::Ty> {
        Val::from_value(self.addr.value())
    }
}

impl_into_var_primitive! {
    u8, u16, u32, u64, usize,
    i8, i16, i32, i64, isize,
    bool,
}

impl<T> AsVal for Val<T> {
    type Ty = T;
    fn as_val(&self, _ctx: &mut FnCtx) -> Val<Self::Ty> {
        *self
    }
}
