use cranelift::prelude::AbiParam;

use crate::for_all_tuples;
use crate::primitive::Primitive;

pub trait ToAbiParams {
    fn to_abi_params(params: &mut Vec<AbiParam>);
}

impl ToAbiParams for () {
    fn to_abi_params(_params: &mut Vec<AbiParam>) {}
}

impl<T> ToAbiParams for &[T] {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        // a slice consist of the len and a pointer to the data
        usize::to_abi_params(params);
        usize::to_abi_params(params);
    }
}

impl<T> ToAbiParams for *mut T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        params.push(AbiParam::new(<*mut T>::ty()));
    }
}

impl<T> ToAbiParams for *const T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        params.push(AbiParam::new(<*const T>::ty()));
    }
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
    usize,
    isize,
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

for_all_tuples!(impl_to_abi_params_tuples);
