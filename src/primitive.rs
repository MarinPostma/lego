use cranelift::prelude::Type;
use cranelift::prelude::types::*;

pub trait ToPrimitive {
    fn to_i64(self) -> i64;
    fn ty() -> Type;
}

macro_rules! primitive_jit_ty {
    ($($src:ident => $dst:ident $(,)?)*) => {
        $(
            impl ToPrimitive for $src {
                fn to_i64(self) -> i64 {
                    // FIXME: This is probably not good wrt signed integers
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
