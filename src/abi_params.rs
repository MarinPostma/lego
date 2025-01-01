use cranelift::prelude::AbiParam;

use crate::primitive::ToPrimitive;

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
