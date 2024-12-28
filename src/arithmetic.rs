use std::ops::Add;

use cranelift::prelude::{InstBuilder as _, TrapCode, Value};

use crate::types::{Val, Var};
use crate::func::{with_ctx, FnCtx};

trait IntAdd {
    fn add(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value;
}

#[allow(private_bounds)]
pub trait Integer: IntAdd {}

macro_rules! impl_integer {
    ($($ty:ident $(,)?)*) => {
        $(impl Integer for $ty {})*
    };
}

impl_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_unsigned_add {
    ($($ty:ident $(,)?)*) => {
        $(
            impl IntAdd for $ty {
                fn add(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    ctx.builder.ins().uadd_overflow_trap(lhs, rhs, TrapCode::INTEGER_OVERFLOW)
                }
            }
        )*
    };
}

impl_unsigned_add!(u8, u16, u32, u64);

macro_rules! impl_signed_add {
    ($($ty:ident $(,)?)*) => {
        $(
            impl IntAdd for $ty {
                fn add(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    ctx.builder.ins().iadd(lhs, rhs)
                }
            }
        )*
    };
}

impl_signed_add!(i8, i16, i32, i64);

impl<T: IntAdd> Add for Var<T> {
    type Output = Val<T>;

    fn add(self, rhs: Self) -> Self::Output {
        with_ctx(|ctx| -> Val<T> {
            let lhs = ctx.builder.use_var(self.variable());
            let rhs = ctx.builder.use_var(rhs.variable());
                
            Val::new(T::add(ctx, lhs, rhs))
        })
    }
}
