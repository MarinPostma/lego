use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();


    let before = Instant::now();

    let main = ctx.func::<(&&str, &&str), i32>(|(_s1, _s2)| {
        lego!({
            if true {
                if false {
                    return Val::new(12i32)
                }
            }

            Val::new(23i32)
        })
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call((&"helloooooo", &"balbal")));
    dbg!(before.elapsed());
}
