
use std::fmt::Display;
use std::ops::MulAssign;
use std::str::FromStr;

use lego::ffi::{Bottom, Function, ToFFIFunctionParams, ToFFIParams};
use lego::prelude::*;

fn do_pow_spec<T>(ctx: &mut Ctx, n: usize)
where
    T: Param + Primitive + FromStr + ToFFIParams + Display + IntMul,
    T::Out<Bottom>: ToFFIFunctionParams,
    T::Ty: AsVal<Ty = T> + Copy,
    Var<T>: MulAssign<T::Ty>,
{
    let my_pow = ctx.func::<T, T>(|x| {
        let mut out = Var::new(x);
        for _ in 1..n {
            out *= x;
        }

        out.value()
    });

    let x = std::env::args().nth(2).unwrap().parse::<T>().unwrap_or_else(|_| panic!());
    let res = ctx.get_compiled_function(my_pow).call(x);
    println!("{res}");
}

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let spec = std::env::args().nth(3).unwrap();

    let n = std::env::args().nth(1).unwrap().parse::<usize>().unwrap_or_else(|_| panic!());

    match spec.as_str() {
        "u64" => do_pow_spec::<u64>(&mut ctx, n),
        "i32" => do_pow_spec::<i32>(&mut ctx, n),
        _ => panic!("unsupported specialization"),
    }
}
fn do_pow_spec<T>(ctx: &mut Ctx, n: usize)
where
    T: Param + Primitive + FromStr + ToFFIParams + Display + IntMul,
    T::Out<Bottom>: ToFFIFunctionParams,
    T::Ty: AsVal<Ty = T> + Copy,
    Var<T>: MulAssign<T::Ty>,
{
    let my_pow = ctx.func::<T, T>(|x| {
        let mut out = Var::new(x);
        for _ in 1..n {
            out *= x;
        }

        out.value()
    });

    let x = std::env::args().nth(2).unwrap().parse::<T>().unwrap_or_else(|_| panic!());
    let res = ctx.get_compiled_function(my_pow).call(x);
    println!("{res}");
}

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let spec = std::env::args().nth(3).unwrap();

    let n = std::env::args().nth(1).unwrap().parse::<usize>().unwrap_or_else(|_| panic!());

    match spec.as_str() {
        "u64" => do_pow_spec::<u64>(&mut ctx, n),
        "i32" => do_pow_spec::<i32>(&mut ctx, n),
        _ => panic!("unsupported specialization"),
    }
}
