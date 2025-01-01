use std::marker::PhantomData;

use cranelift::prelude::Value;
use cranelift::prelude::InstBuilder as _;

use crate::func::with_ctx;
use crate::func::FnCtx;
use crate::primitive::ToPrimitive;

pub struct Val<T> {
    value: Value,
    _pth: PhantomData<T>,
}

impl<T> Copy for Val<T> {}
impl<T> Clone for Val<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Val<T> {
    pub fn new(val: T) -> Val<T>
    where T: ToPrimitive,
    {
        with_ctx(|ctx| {
            let val = ctx.builder().ins().iconst(T::ty(), val.to_i64());
            Val::from_value(val)
        })
    }

    pub(crate) fn from_value(value: Value) -> Self {
        Self { value, _pth: PhantomData }
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }
}

pub trait AsVal {
    type Ty;

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

impl_into_var_primitive! {
    u8, u16, u32, u64,
    i8, i16, i32, i64,
}

impl<T> AsVal for Val<T> {
    type Ty = T;
    fn as_val(&self, _ctx: &mut FnCtx) -> Val<Self::Ty> {
        *self
    }
}
