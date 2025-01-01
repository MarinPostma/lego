use std::collections::HashMap;
use std::time::Instant;

use lego::ctx::Ctx;
use lego::func::Call as _;
use lego::prelude::*;
use lego_macros::LegoBlock;

#[derive(LegoBlock, Debug)]
struct Foo {
    y: u32,
    x: u32,
}

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    // we can probably flip the traits so that we can let rust do the type inference?
    // TODO: I don't think we need to name the function
    let main = ctx.func::<&str, u32>("main", |s| {
        let func = (|hello: &str| { println!("hello: {hello}"); 0 }).into_host_fn();
        func.call(s.get_ref())
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);


    let before = Instant::now();
    dbg!(main.call("helloooooo"));
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
