use std::marker::PhantomData;

use cranelift::prelude::{AbiParam, InstBuilder, IntCC, Type, Value};
use cranelift::prelude::types::*;
use cranelift_frontend::Variable;

use crate::func::{with_ctx, FnCtx};

pub trait ToJitPrimitive {
    fn to_i64(self) -> i64;
    fn ty() -> Type;
}

pub trait ToAbiParams {
    fn to_abi_params(params: &mut Vec<AbiParam>);
}

impl ToAbiParams for () {
    fn to_abi_params(_params: &mut Vec<AbiParam>) { }
}

macro_rules! impl_to_abi_params_primitive {
    ($($ty:ident $(,)?)*) => {
        $(
            impl ToAbiParams for $ty {
                fn to_abi_params(params: &mut Vec<AbiParam>) {
                    params.push(AbiParam::new($ty::ty()));
                }
            }
        )*
    };
}

impl_to_abi_params_primitive! {
    i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64,
}

macro_rules! primitive_jit_ty {
    ($($src:ident => $dst:ident $(,)?)*) => {
        $(
            impl ToJitPrimitive for $src {
                fn to_i64(self) -> i64 {
                    self as i64
                }

                fn ty() -> Type {
                    $dst
                }
            }
        )*
    };
}

primitive_jit_ty! {
    u8 => I8,
    i8 => I8,
    u16 => I16,
    i16 => I16,
    u32 => I32,
    i32 => I32,
    i64 => I64,
    u64 => I64,
}

macro_rules! impl_to_abi_params_tuples {
    ($($ty:ident $(,)?)*) => {
        impl<$($ty,)*> ToAbiParams for ($($ty,)*)
        where 
            $($ty: ToAbiParams,)*
        {
            fn to_abi_params(params: &mut Vec<AbiParam>) {
                $(
                    $ty::to_abi_params(params);
                )*
            }
        }
    };
}

impl_to_abi_params_tuples!(A, B);
impl_to_abi_params_tuples!(A, B, C);
impl_to_abi_params_tuples!(A, B, C, D);
impl_to_abi_params_tuples!(A, B, C, D, E);
impl_to_abi_params_tuples!(A, B, C, D, E, F);
impl_to_abi_params_tuples!(A, B, C, D, E, F, G);

#[derive(Copy, Clone)]
pub struct Var<T> {
    variable: Variable,
    _pth: PhantomData<T>,
}

impl<T> Var<T> {
    pub fn new(variable: Variable) -> Self {
        Self { variable, _pth: PhantomData }
    }

    pub(crate) fn variable(&self) -> Variable {
        self.variable
    }
}

#[derive(Copy, Clone)]
pub struct Val<T> {
    value: Value,
    _pth: PhantomData<T>,
}

impl<T, U> Compare<U> for T 
where
    T: IntoVal<Ty = u32>,
    U: IntoVal<Ty = u32>,
{
    fn eq(self, other: U) -> Val<bool> {
        with_ctx(|ctx| {
            let lhs = self.into_val(ctx);
            let rhs = other.into_val(ctx);
            let val = ctx.builder().ins().icmp(IntCC::Equal, lhs.value(), rhs.value());
            Val::new(val)
        })
    }
}

pub trait Compare<Rhs = Self> {
    fn eq(self, other: Rhs) -> Val<bool>;
}

impl<T> Val<T> {
    pub fn new(value: Value) -> Self {
        Self { value, _pth: PhantomData }
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }
}

pub trait IntoVal {
    type Ty;

    fn into_val(self, ctx: &mut FnCtx) -> Val<Self::Ty>;
}

impl<T> IntoVal for Var<T> {
    type Ty = T;
    fn into_val(self, ctx: &mut FnCtx) -> Val<T> {
        let val = ctx.builder.use_var(self.variable);
        Val::new(val)
    }
}

impl IntoVal for u64 {
    type Ty = u64;
    fn into_val(self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        let value =ctx.builder.ins().iconst(Self::ty(), self as i64);
        Val::new(value)
    }
}

impl<T> IntoVal for Val<T> {
    type Ty = T;
    fn into_val(self, _ctx: &mut FnCtx) -> Val<Self::Ty> {
        self
    }
}
