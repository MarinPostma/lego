use std::marker::PhantomData;
use std::ops::{AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, DivAssign, MulAssign, RemAssign, SubAssign};

use cranelift_frontend::Variable;

use crate::arithmetic::{IntAdd, IntBitAnd, IntBitOr, IntBitXor, IntDiv, IntMul, IntRem, IntSub};
use crate::func::{with_ctx, FnCtx};
use crate::primitive::Primitive;
use crate::val::{AsVal, Val};

#[derive(Copy, Clone)]
pub struct Var<T> {
    variable: Variable,
    _pth: PhantomData<T>,
}

impl<T> Var<T> {
    pub fn new<V>(v: V) -> Self
    where
        V: AsVal<Ty = T>,
        T: Primitive,
    {
        with_ctx(|ctx| {
            let var = ctx.declare_var();
            ctx.builder().declare_var(var, T::ty());
            let val = v.as_val(ctx);
            ctx.builder().def_var(var, val.value());
            Self::from_variable(var)
        })
    }

    pub fn assign(&mut self, val: impl AsVal<Ty = T>) {
        with_ctx(|ctx| {
            self.assign_ctx(ctx, val);
        })
    }

    fn assign_ctx(&mut self, ctx: &mut FnCtx, val: impl AsVal<Ty = T>) {
        let value = val.as_val(ctx);
        ctx.builder().def_var(self.variable(), value.value());
    }

    pub(crate) fn from_variable(variable: Variable) -> Self {
        Self {
            variable,
            _pth: PhantomData,
        }
    }

    pub(crate) fn variable(&self) -> Variable {
        self.variable
    }
}

impl<T> AsVal for Var<T> {
    type Ty = T;
    fn as_val(&self, ctx: &mut FnCtx) -> Val<T> {
        let val = ctx.builder.use_var(self.variable);
        Val::from_value(val)
    }
}

macro_rules! impl_assign {
    ($op:ident, $trait:ident, $f:ident) => {
        impl<U, V> $op<U> for Var<V>
        where
            U: AsVal<Ty = V>,
            V: $trait,
        {
            fn $f(&mut self, rhs: U) {
                with_ctx(|ctx| {
                    let lhs = self.as_val(ctx).value();
                    let rhs = rhs.as_val(ctx).value();
                    let new_val = V::perform(ctx, lhs, rhs);
                    self.assign_ctx(ctx, Val::from_value(new_val));
                })
            }
        }
    };
}

impl_assign!(AddAssign, IntAdd, add_assign);
impl_assign!(SubAssign, IntSub, sub_assign);
impl_assign!(MulAssign, IntMul, mul_assign);
impl_assign!(DivAssign, IntDiv, div_assign);
impl_assign!(RemAssign, IntRem, rem_assign);
impl_assign!(BitOrAssign, IntBitOr, bitor_assign);
impl_assign!(BitAndAssign, IntBitAnd, bitand_assign);
impl_assign!(BitXorAssign, IntBitXor, bitxor_assign);
