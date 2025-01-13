use core::fmt;

use super::func::CompiledFunc;

pub trait Function {
    type FFIFn: ToFFIFunctionParams;
    type Params;
    type Result;

    fn call(&self, params: Self::Params) -> Self::Result;
}

#[derive(Debug)]
pub struct Bottom;

pub trait ToFFIFunctionParams: fmt::Debug {
    unsafe fn call<R>(self, f: *const u8) -> R;
}

pub trait ToFFIParams: fmt::Debug {
    type Out<T: fmt::Debug>: std::fmt::Debug;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T>;
}

trait Primitive: fmt::Debug {}

impl Primitive for u64 {}
impl Primitive for i32 {}
impl Primitive for usize {}
impl<T: fmt::Debug> Primitive for &[T] {}

impl<T: Primitive> ToFFIFunctionParams for Param<Bottom, T> {
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(_, a) = self;
        let f = std::mem::transmute::<*const u8, extern "C" fn(T) -> R>(f);
        f(a)
    }
}

impl<T: Primitive + fmt::Debug, U: Primitive + fmt::Debug> ToFFIFunctionParams for Param<Param<Bottom, T>, U> {
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(_, a), b) = self;
        let f = std::mem::transmute::<*const u8, extern "C" fn(U, T) -> R>(f);
        f(b, a)
    }
}

impl<T: Primitive, U: Primitive, V: Primitive> ToFFIFunctionParams
    for Param<Param<Param<Bottom, T>, U>, V>
{
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(Param(_, a), b), c) = self;
        let f = std::mem::transmute::<*const u8, extern "C" fn(V, U, T) -> R>(f);
        f(c, b, a)
    }
}

impl<T: Primitive + fmt::Debug, U: Primitive + fmt::Debug, V: Primitive + fmt::Debug, W: Primitive + fmt::Debug> ToFFIFunctionParams
    for Param<Param<Param<Param<Bottom, T>, U>, V>, W>
{
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(Param(Param(_, a), b), c), d) = self;
        let f = std::mem::transmute::<*const u8, extern "C" fn(W, V, U, T) -> R>(f);
        f(d, c, b, a)
    }
}

#[derive(Debug)]
pub struct Param<T, U>(T, U);

pub trait ToTuple {
    type Output;
}

impl ToFFIParams for u64 {
    type Out<T: fmt::Debug> = Param<T, u64>;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl ToFFIParams for i32 {
    type Out<T: fmt::Debug> = Param<T, i32>;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl ToFFIParams for usize {
    type Out<T: fmt::Debug> = Param<T, usize>;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl<T: fmt::Debug> ToFFIParams for &[T] {
    type Out<U: fmt::Debug> = <usize as ToFFIParams>::Out<<usize as ToFFIParams>::Out<U>>;

    fn to_ffi_params<U: fmt::Debug>(self, t: U) -> Self::Out<U> {
        Param(Param(t, self.as_ptr() as usize), self.len())
    }
}

impl<A, B> ToFFIParams for (A, B)
where
    A: ToFFIParams,
    B: ToFFIParams,
{
    type Out<T: fmt::Debug> = A::Out<B::Out<T>>;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T> {
        self.0.to_ffi_params(self.1.to_ffi_params(t))
    }
}

impl<A, B, C> ToFFIParams for (A, B, C)
where
    A: ToFFIParams,
    B: ToFFIParams,
    C: ToFFIParams,
{
    type Out<T: fmt::Debug> = A::Out<B::Out<C::Out<T>>>;

    fn to_ffi_params<T: fmt::Debug>(self, t: T) -> Self::Out<T> {
        self.0
            .to_ffi_params(self.1.to_ffi_params(self.2.to_ffi_params(t)))
    }
}

impl<A, R> Function for CompiledFunc<'_, A, R>
where
    A: ToFFIParams,
    A::Out<Bottom>: ToFFIFunctionParams,
{
    type Params = A;
    type FFIFn = A::Out<Bottom>;
    type Result = R;

    fn call(&self, params: Self::Params) -> Self::Result {
        let params = params.to_ffi_params(Bottom);
        // safety: CompiledFunction guarantees the provenance of the function pointer, and we
        // correct type is asserted at compile time.
        unsafe { params.call(self.ptr) }
    }
}
