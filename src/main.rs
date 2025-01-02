use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();


    let before = Instant::now();

    let main = ctx.func::<(&&str, &&str), ()>("main", |(_s1, _s2)| {
        lego_macros::lego!({
            if Val::new(true) {
                println!("hello1");
                1i32
            } else {
                println!("!hello");
                1i32
            }
        });
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call((&"helloooooo", &"balbal")));
    dbg!(before.elapsed());
}
