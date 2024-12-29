use std::fmt::Display;

use lego::types::{Val, Var};
use lego::host_fns;
use lego::func::{host_fn, Call as _, Func, Param, Results};
use lego::ctx::Ctx;
use lego::arithmetic::Integer;
use lego_macros::LegoBlock;

extern "C" fn print_hello<T: Display>(t: T) {
    println!("hello world: {t}");
}

fn add_any<T>(name: &str, ctx: &mut Ctx) -> Func<(T, T), T>
where 
    T: Param<Ty = Var<T>> + Results<Results = Val<T>> + Integer
{
    ctx.func::<(T, T), T>(name, |(x, y)| {
        x + y
    })
}

#[derive(LegoBlock, Debug)]
struct Test {
    y: u64,
    x: u64,
}

fn main() {
    let mut builder = Ctx::builder();
    // this is quite ugly, I wish there was a better way to pass host functions
    builder.register_host_functions(host_fns![
        print_hello::<u32> => extern "C" fn(u32),
        print_hello::<u64> => extern "C" fn(u64),
    ]);

    let mut ctx = builder.build();

    let add = add_any::<u32>("add", &mut ctx);

    let main = ctx.func::<(u32, &mut Test), u32>("main", |(x, test)| {
        let p_u32 = host_fn(print_hello::<u32> as extern "C" fn(u32));
        let p_u64 = host_fn(print_hello::<u64> as extern "C" fn(u64));
        p_u32.call(x);
        p_u64.call(test.x().get());
        p_u64.call(test.y().get());
        test.y_mut().put(1);
        add.call((x, x))
    });

    let main = ctx.get_compiled_function(main);

    let mut test = Test {
        x: 1232,
        y: 125,
    };

    dbg!(main.call((1, &mut test)));

    dbg!(test);
}
