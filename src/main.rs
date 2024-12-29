use std::collections::HashMap;
use std::fmt::Display;

use lego::arithmetic::Integer;
use lego::ctx::Ctx;
use lego::func::{Call as _, Func, Param, Results};
use lego::types::{Val, Var};
use lego_macros::LegoBlock;

extern "C" fn print_hello<T: Display>(t: T) {
    println!("hello world: {t}");
}

fn add_any<T>(name: &str, ctx: &mut Ctx) -> Func<(T, T), T>
where
    T: Param<Ty = Var<T>> + Results<Results = Val<T>> + Integer,
{
    ctx.func::<(T, T), T>(name, |(x, y)| x + y)
}

#[derive(LegoBlock, Debug)]
struct Foo {
    y: u32,
    x: u32,
}

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let main = ctx.func::<(&Foo, &mut HashMap<u32, u32>), u32>("main", |(foo, mut map)| {
        map.insert(foo.x() + foo.y(), foo.x().get());
        foo.x().get()
    });

    let main = ctx.get_compiled_function(main);

    let mut map = HashMap::new();
    let foo = Foo {
        y: 42,
        x: 1337,
    };

    dbg!(main.call((&foo, &mut map)));

    dbg!(map);
}
