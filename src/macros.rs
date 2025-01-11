//! Define a bunch of useful macros for use everywhere
#[macro_export]
#[doc(hidden)]
macro_rules! map_ident {
    ($f:ident: $($id:ident $(,)?)*) => {
        $($f!($id);)*
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! for_all_primitives {
    ($cb:ident) => {
        $crate::map_ident!($cb: i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! for_all_tuples {
    ($cb:ident) => {
        $cb!(A);
        $cb!(A, B);
        $cb!(A, B, C);
        $cb!(A, B, C, D);
        $cb!(A, B, C, D, E);
        $cb!(A, B, C, D, E, F);
        $cb!(A, B, C, D, E, F, G);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! maybe_paren {
    ($ty:ty) => {
        $ty
    };
    ($($ty:ty $(,)?)*) => {
        ($($ty),*)
    };
}
