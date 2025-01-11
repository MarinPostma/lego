use core::fmt;

use super::func::CompiledFunc;

pub trait Function {
    type FFIFn: ToFFIFunctionParams;
    type Params;
    type Result;

    fn call(&self, params: Self::Params) -> Self::Result;
}

pub struct Bottom;

pub trait ToFFIFunctionParams {
    unsafe fn call<R>(self, f: *const u8) -> R;
}

pub trait ToFFIParams: fmt::Debug {
    type Out<T>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T>;
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

impl<T: Primitive, U: Primitive> ToFFIFunctionParams for Param<Param<Bottom, T>, U> {
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(_, a), b) = self;
        let f = std::mem::transmute::<*const u8, extern "C" fn(T, U) -> R>(f);
        f(a, b)
    }
}

impl<T: Primitive, U: Primitive, V: Primitive> ToFFIFunctionParams
    for Param<Param<Param<Bottom, T>, U>, V>
{
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(Param(_, a), b), c) = self;
        dbg!((&a, &b, &c));
        let f = std::mem::transmute::<*const u8, extern "C" fn(T, U, V) -> R>(f);
        f(a, b, c)
    }
}

impl<T: Primitive, U: Primitive, V: Primitive, W: Primitive> ToFFIFunctionParams
    for Param<Param<Param<Param<Bottom, T>, U>, V>, W>
{
    unsafe fn call<R>(self, f: *const u8) -> R {
        let Param(Param(Param(Param(_, a), b), c), d) = self;
        dbg!((&a, &b, &c, &d));
        let f = std::mem::transmute::<*const u8, extern "C" fn(W, V, U, T) -> R>(f);
        f(d, c, b, a)
    }
}

pub struct Param<T, U>(T, U);

pub trait ToTuple {
    type Output;
}

impl ToFFIParams for u64 {
    type Out<T> = Param<T, u64>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl ToFFIParams for i32 {
    type Out<T> = Param<T, i32>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl ToFFIParams for usize {
    type Out<T> = Param<T, usize>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T> {
        Param(t, self)
    }
}

impl<T: fmt::Debug> ToFFIParams for &[T] {
    type Out<U> = <usize as ToFFIParams>::Out<<usize as ToFFIParams>::Out<U>>;

    fn to_ffi_params<U>(self, t: U) -> Self::Out<U> {
        Param(Param(t, self.as_ptr() as usize), self.len())
    }
}

impl<A, B> ToFFIParams for (A, B)
where
    A: ToFFIParams,
    B: ToFFIParams,
{
    type Out<T> = A::Out<B::Out<T>>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T> {
        self.0.to_ffi_params(self.1.to_ffi_params(t))
    }
}

impl<A, B, C> ToFFIParams for (A, B, C)
where
    A: ToFFIParams,
    B: ToFFIParams,
    C: ToFFIParams,
{
    type Out<T> = A::Out<B::Out<C::Out<T>>>;

    fn to_ffi_params<T>(self, t: T) -> Self::Out<T> {
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
