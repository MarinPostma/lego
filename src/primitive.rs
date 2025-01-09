use cranelift::prelude::Type;
use cranelift::prelude::types::*;

pub trait Primitive {
    fn to_i64(self) -> i64;
    fn ty() -> Type;
}

impl<T: Sized> Primitive for &T {
    fn to_i64(self) -> i64 {
        self as *const T as usize as i64
    }

    fn ty() -> Type {
        Type::int_with_byte_size(size_of::<Self>() as u16).unwrap()
    }
}

impl<T: Sized> Primitive for &mut T {
    fn to_i64(self) -> i64 {
        self as *const T as usize as i64
    }

    fn ty() -> Type {
        Type::int_with_byte_size(size_of::<Self>() as u16).unwrap()
    }
}

impl<T: Sized> Primitive for *mut T {
    fn to_i64(self) -> i64 {
        self as usize as i64
    }

    fn ty() -> Type {
        Type::int_with_byte_size(size_of::<Self>() as u16).unwrap()
    }
}

impl<T: Sized> Primitive for *const T {
    fn to_i64(self) -> i64 {
        self as usize as i64
    }

    fn ty() -> Type {
        Type::int_with_byte_size(size_of::<Self>() as u16).unwrap()
    }
}

impl Primitive for bool {
    fn to_i64(self) -> i64 {
        if self {
            1
        } else {
            0
        }
    }

    fn ty() -> Type {
        I8
    }
}


macro_rules! primitive_jit_ty {
    ($($src:ident $(,)?)*) => {
        $(
            impl Primitive for $src {
                fn to_i64(self) -> i64 {
                    // FIXME: This is probably not good wrt signed integers
                    self as i64
                }

                fn ty() -> Type {
                    Type::int_with_byte_size(std::mem::size_of::<Self>() as u16).unwrap()
                }
            }
        )*
    };
}

primitive_jit_ty! {
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
    i64,
    u64,
    usize,
    isize,
}
