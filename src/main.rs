use std::collections::HashMap;
use std::time::Instant;

use lego::ctx::Ctx;
use lego::func::Call as _;
use lego::var::Var;
use lego::prelude::*;
use lego_macros::LegoBlock;
use lego_macros::{lego_while, lego_if};

#[derive(LegoBlock, Debug)]
struct Foo {
    y: u32,
    x: u32,
}

fn say_hello(x: u32) -> u32 {
    println!("hello: {x}");
    x
}

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    let main = ctx.func::<(&Foo, &mut HashMap<u32, u32>), u32>("main", |(f, mut map)| {
        let func = say_hello.into_host_fn();
        func.call(f.x().get())
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let mut map = HashMap::new();
    let f = Foo {
        y: 1,
        x: 0,
    };


    let before = Instant::now();
    dbg!(main.call((&f, &mut map)));
    dbg!(before.elapsed());

    dbg!(&map);

    let mut map = HashMap::new();
    let before = Instant::now();
    dbg!(native(&f, &mut map));
    dbg!(before.elapsed());
}

fn native(f: &Foo, map: &mut HashMap<u32, u32>) -> u32 {
    let mut v = 0;
    while v != 10 {
        if v == 5 {
            map.insert(v, v + 100);
        } else {
            map.insert(v, v);
        }

        v += 1;
    }

    f.x
}
