use std::collections::HashMap;
use std::time::Instant;

use lego::ctx::Ctx;
use lego::func::Call as _;
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
    let main = ctx.func::<(&Foo, &mut HashMap<u32, u32>), u32>("main", |(f, mut map)| {
        map.insert(f.x() * 20 + f.y() + 10, f.x().get());
        f.x() * 2
    });
    dbg!(before.elapsed());

    let before = Instant::now();
    let main = ctx.get_compiled_function(main);

    let mut map = HashMap::new();
    let f = Foo {
        y: 42,
        x: 1337,
    };

    dbg!(&map);

    dbg!(main.call((&f, &mut map)));
    dbg!(before.elapsed());

    dbg!(map);
}
