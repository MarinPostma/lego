use std::marker::PhantomData;

use cranelift_frontend::Variable;
use cranelift::prelude::*;

use crate::val::{AsVal, Val};
use crate::primitive::ToPrimitive;
use crate::func::{with_ctx, FnCtx, IntoParams};

#[derive(Copy, Clone)]
pub struct Var<T> {
    variable: Variable,
    _pth: PhantomData<T>,
}

impl<T> Var<T> {
    pub fn new(v: T) -> Self
    where T: ToPrimitive
    {
        with_ctx(|ctx| {
            let var = ctx.declare_var();
            ctx.builder().declare_var(var, T::ty());
            let val = ctx.builder().ins().iconst(T::ty(), v.to_i64());
            ctx.builder().def_var(var, val);
            Self::from_variable(var)
        })
    }

    pub fn assign(&mut self, val: impl AsVal<Ty = T>)
    {
        with_ctx(|ctx| {
            let value = val.as_val(ctx);
            ctx.builder().def_var(self.variable(), value.value());
        })
    }

    pub(crate) fn from_variable(variable: Variable) -> Self {
        Self { variable, _pth: PhantomData }
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

impl<T> IntoParams for Var<T> {
    type Input = T;

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        let val = ctx.builder.use_var(self.variable());
        out.push(val);
    }
}
