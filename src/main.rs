use std::collections::HashMap;
use std::fmt::Display;

use lego::types::{Val, Var};
use lego::host_fns;
use lego::func::{Call as _, Func, Param, Results};
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

    let main = ctx.func::<(u32, &mut HashMap<u32, u32>), u32>("main", |(x, mut map)| {
        map.insert(x, x);
        x + x
    });

    let main = ctx.get_compiled_function(main);

    let mut map = HashMap::new();

    dbg!(main.call((1, &mut map)));

    dbg!(map);
}
