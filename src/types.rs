use std::marker::PhantomData;

use cranelift::prelude::{AbiParam, Type, Value};
use cranelift::prelude::types::*;
use cranelift_frontend::Variable;

pub trait ToJitPrimitive {
    fn ty() -> Type;
}

pub trait ToAbiParams {
    fn to_abi_params(params: &mut Vec<AbiParam>);
}

impl ToAbiParams for () {
    fn to_abi_params(_params: &mut Vec<AbiParam>) { }
}

impl<T: ToJitPrimitive> ToAbiParams for T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        params.push(AbiParam::new(T::ty()));
    }
}

macro_rules! primitive_jit_ty {
    ($($src:ident => $dst:ident $(,)?)*) => {
        $(
            impl ToJitPrimitive for $src {
                fn ty() -> Type {
                    $dst
                }
            }
        )*
    };
}

primitive_jit_ty! {
    u32 => I32,
    i32 => I32,
    i64 => I64,
    u64 => I64,
}

impl<A, B> ToAbiParams for (A, B)
where
    A: ToAbiParams,
    B: ToAbiParams,
{
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        A::to_abi_params(params);
        B::to_abi_params(params);
    }
}

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

impl<T> Val<T> {
    pub fn new(value: Value) -> Self {
        Self { value, _pth: PhantomData }
    }

    pub(crate) fn value(&self) -> Value {
        self.value
    }
}
