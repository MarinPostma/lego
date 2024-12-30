use std::collections::HashMap;
use std::time::Instant;

use lego::ctx::Ctx;
use lego::func::Call as _;
use lego::lego_if;
use lego::types::Compare as _;
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
    let main = ctx.func::<(&Foo, &mut HashMap<u32, u32>), u32>("main", |(f, map)| {
        let val = lego_if! {
            if (f.y().eq(f.x() + 1)) {
                f.x() + 1
            } else {
                f.x() + 2
            }
        };

        val
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let mut map = HashMap::new();
    let f = Foo {
        y: 1,
        x: 0,
    };

    dbg!(&map);

    let before = Instant::now();
    dbg!(main.call((&f, &mut map)));
    dbg!(before.elapsed());

    let before = Instant::now();
    dbg!(native(&f, &mut map));
    dbg!(before.elapsed());
}

fn native(f: &Foo, map: &mut HashMap<u32, u32>) -> u32 {
    let x = f.x;
    let y = f.y;

    let val = if y == x + 1 {
        x + 1
    } else {
        x + 2
    };

    val
}
