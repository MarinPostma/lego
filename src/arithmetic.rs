use std::ops::{Add, Mul};

use cranelift::prelude::{InstBuilder as _, TrapCode, Value};

use crate::types::{IntoVal, ToJitPrimitive, Val, Var};
use crate::func::{with_ctx, FnCtx};
use crate::Proxy;

trait IntAdd: ToJitPrimitive {
    fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value;
}

trait IntMul: ToJitPrimitive {
    fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value;
}

#[allow(private_bounds)]
pub trait Integer: IntAdd {}

macro_rules! impl_integer {
    ($($ty:ident $(,)?)*) => {
        $(impl Integer for $ty {})*
    };
}

impl_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_unsigned {
    ($($ty:ident $(,)?)*) => {
        $(
            impl IntAdd for $ty {
                fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    ctx.builder.ins().uadd_overflow_trap(lhs, rhs, TrapCode::INTEGER_OVERFLOW)
                }
            }

            impl IntMul for $ty {
                fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    ctx.builder().ins().imul(lhs, rhs)
                }
            }
        )*
    };
}

impl_unsigned!(u8, u16, u32, u64);

macro_rules! impl_signed_add {
    ($($ty:ident $(,)?)*) => {
        $(
            impl IntAdd for $ty {
                fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    ctx.builder.ins().iadd(lhs, rhs)
                }
            }

            impl IntMul for $ty {
                fn perform(_ctx: &mut FnCtx, _lhs: Value, _rhs: Value) -> Value {
                    todo!()
                    // ctx.builder().ins().smul_overflow(lhs, rhs)
                }
            }
        )*
    };
}

impl_signed_add!(i8, i16, i32, i64);

// macro from hell?
// We can't implement Add for all T that implement IntoVal
macro_rules! impl_op {
    ($op:ident, $bound:ident, $f:ident => [$($ty:ident $(,)?)*]) => {
        $(
            impl<T: $bound> $op<T> for $ty<T> {
                type Output = Val<T>;

                fn $f(self, rhs: T) -> Self::Output {
                    with_ctx(|ctx| -> Val<T> {
                        let lhs = self.into_val(ctx);
                        let rhs = ctx.builder().ins().iconst(T::ty(), rhs.to_i64());
                        Val::new(T::perform(ctx, lhs.value(), rhs))
                    })
                }
            }
        )*

        impl_op! { @recur_left $op, $bound, $f => [$($ty),*], [$($ty),*] }
    };

    // recurse left
    (@recur_left $op:ident, $bound:ident, $f:ident => [$lhs:ident, $($rest:ident $(,)?)*], [$($rhs:ident $(,)?)*]) => {
        impl_op! {@impl $op, $bound, $f; $lhs => [$($rhs),*] }
        impl_op! { @recur_left $op, $bound, $f => [$($rest),*], [$($rhs),*] }
    };
    // base case left
    (@recur_left $op:ident, $bound:ident, $f:ident => [$lhs:ident $(,)?], [$($rhs:ident $(,)?)*]) => {
        impl_op! {@impl $op, $bound, $f; $lhs => [$($rhs),*] }
    };

    (@impl $op:ident, $bound:ident, $f:ident; $lhs:ident => [$($rhs:ident $(,)?)*]) => {
        $(
            impl<T: $bound> $op<$lhs<T>> for $rhs<T> {
                type Output = Val<T>;
            
                fn $f(self, rhs: $lhs<T>) -> Self::Output {
                    with_ctx(|ctx| -> Val<T> {
                        let lhs = self.into_val(ctx);
                        let rhs = rhs.into_val(ctx);
                        Val::new(T::perform(ctx, lhs.value(), rhs.value()))
                    })
                }
            }
        )*
    };
}

impl_op! { Add, IntAdd, add => [Var, Val, Proxy] }
impl_op! { Mul, IntMul, mul => [Var, Val, Proxy] }
