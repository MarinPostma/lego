use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();

    let main = ctx.func::<i32, i32>(|val| {
        ControlFlow::Ret(
            lego!({
                if true {
                    if val.eq(&42i32) {
                        return Val::new(12i32)
                    }
                }

                val + 1i32
            })
        )
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call(41));
    dbg!(before.elapsed());
}
