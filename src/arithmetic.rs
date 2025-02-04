use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub};

use cranelift::prelude::{InstBuilder, Value};

use crate::func::{with_ctx, FnCtx};
use crate::primitive::Primitive;
use crate::val::{AsVal, Val};
use crate::var::Var;
use crate::{for_all_primitives, map_ident};

macro_rules! make_arithmetic_traits {
    ($($name:ident $(,)?)*) => {
        $(
            pub trait $name: Primitive {
                fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value;
            }
        )*
    };
}

make_arithmetic_traits! {
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntRem,
    IntShl,
    IntShr,
    IntBitAnd,
    IntBitOr,
    IntBitXor,
}

macro_rules! impl_arithmetic {
    ($ty:ident: $($name:ident => |$ctx:ident, $lhs:ident, $rhs:ident| $code:expr $(,)?)*) => {
        $(
            impl $name for $ty {
                fn perform(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
                    let $ctx = ctx;
                    let $lhs = lhs;
                    let $rhs = rhs;
                    $code
                }
            }
        )*
    };
}

macro_rules! impl_common {
    ($ty:ident) => {
        impl_arithmetic!($ty:
            IntAdd => |ctx, lhs, rhs| ctx.builder().ins().iadd(lhs, rhs),
            IntSub => |ctx, lhs, rhs| ctx.builder().ins().isub(lhs, rhs),
            IntMul => |ctx, lhs, rhs| ctx.builder().ins().imul(lhs, rhs),
            IntShl => |ctx, lhs, rhs| ctx.builder().ins().ishl(lhs, rhs),
            IntBitAnd => |ctx, lhs, rhs| ctx.builder().ins().band(lhs, rhs),
            IntBitOr => |ctx, lhs, rhs| ctx.builder().ins().bor(lhs, rhs),
            IntBitXor => |ctx, lhs, rhs| ctx.builder().ins().bxor(lhs, rhs),
        );
    };
}

macro_rules! impl_signed {
    ($ty:ident) => {
        impl_arithmetic!($ty:
            IntDiv => |ctx, lhs, rhs| ctx.builder().ins().sdiv(lhs, rhs),
            IntRem => |ctx, lhs, rhs| ctx.builder().ins().srem(lhs, rhs),
            IntShr => |ctx, lhs, rhs| ctx.builder().ins().sshr(lhs, rhs),
        );
    };
}

macro_rules! impl_unsigned {
    ($ty:ident) => {
        impl_arithmetic!($ty:
            IntDiv => |ctx, lhs, rhs| ctx.builder().ins().udiv(lhs, rhs),
            IntRem => |ctx, lhs, rhs| ctx.builder().ins().urem(lhs, rhs),
            IntShr => |ctx, lhs, rhs| ctx.builder().ins().ushr(lhs, rhs),
        );
    };
}

for_all_primitives!(impl_common);
map_ident!(impl_signed: i8, i16, i32, i64, isize);
map_ident!(impl_unsigned: u8, u16, u32, u64, usize);

impl BitAnd<Val<bool>> for Val<bool> {
    type Output = Val<bool>;

    fn bitand(self, rhs: Val<bool>) -> Self::Output {
        with_ctx(|ctx| Val::from_value(ctx.builder().ins().band(self.value(), rhs.value())))
    }
}

impl Not for Val<bool> {
    type Output = Val<bool>;

    fn not(self) -> Self::Output {
        with_ctx(|ctx| {
            Val::from_value(ctx.builder.ins().bnot(self.value))
        })
    }
}

// macro from hell?
// We can't implement Add for all T that implement IntoVal
macro_rules! impl_op {
    ($op:ident, $bound:ident, $f:ident => [$($ty:ident $(,)?)*]) => {
        $(
            impl<T: $bound> $op<T> for $ty<T> {
                type Output = Val<T>;

                fn $f(self, rhs: T) -> Self::Output {
                    with_ctx(|ctx| -> Val<T> {
                        let lhs = self.as_val(ctx);
                        let rhs = ctx.builder().ins().iconst(T::ty(), rhs.to_i64());
                        Val::from_value(T::perform(ctx, lhs.value(), rhs))
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
                        let lhs = self.as_val(ctx);
                        let rhs = rhs.as_val(ctx);
                        Val::from_value(T::perform(ctx, lhs.value(), rhs.value()))
                    })
                }
            }
        )*
    };
}

impl_op! { Add, IntAdd, add => [Var, Val] }
impl_op! { Mul, IntMul, mul => [Var, Val] }
impl_op! { Sub, IntSub, sub => [Var, Val] }
impl_op! { Div, IntDiv, div => [Var, Val] }
impl_op! { Rem, IntRem, rem => [Var, Val] }
impl_op! { Shl, IntShl, shl => [Var, Val] }
impl_op! { Shr, IntShr, shr => [Var, Val] }
impl_op! { BitAnd, IntBitAnd, bitand => [Var, Val] }
impl_op! { BitOr, IntBitOr, bitor => [Var, Val] }
impl_op! { BitXor, IntBitXor, bitxor => [Var, Val] }
