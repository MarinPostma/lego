use std::fmt::Display;

use cranelift_lego::types::{ToJitPrimitive, Val};
use cranelift_lego::host_fns;
use cranelift_lego::func::{host_fn, Call as _, Func, Results};
use cranelift_lego::ctx::Ctx;
use cranelift_lego::arithmetic::Integer;


extern "C" fn print_hello<T: Display>(t: T) {
    println!("hello world: {t}");
}

fn add_any<T>(name: &str, ctx: &mut Ctx) -> Func<(T, T), T>
where 
    T: ToJitPrimitive + Results<Results = Val<T>> + Integer
{
    ctx.func::<(T, T), T>(name, |(x, y)| {
        x + y
    })
}

fn main() {
    let mut builder = Ctx::builder();
    builder.register_host_functions(host_fns![
        print_hello::<u32> => extern "C" fn(u32),
    ]);

    let mut ctx = builder.build();

    let add = add_any::<u32>("add", &mut ctx);

    let main = ctx.func::<(u32, u32), u32>("main", |(x, y)| {
        let f = host_fn(print_hello::<u32> as extern "C" fn(u32));
        f.call(x);
        let val = add.call((x, y));
        f.call(val);
        val
    });

    let main = ctx.get_compiled_function(main);

    dbg!(main.call((1, 3)));
}
