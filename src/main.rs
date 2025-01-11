use std::time::Instant;

use lego::ffi::Function;
use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    let main = ctx.func::<usize, i32>(|_val| {
        let print = (|v: usize| println!("val: {v}")).into_host_fn();
        let sum = (0usize..30)
            .into_jiter()
            .filter(|it| (*it % 2usize).eq(0usize.value()))
            .map(|it| it * 2)
            .map(|it| it + 1)
            .map(|it| {
                // print.call(it);
                it / 3
            })
            .fold(Val::new(0usize), |acc, it| acc + it);

        print.call(sum);
        0.value()
    });
    let main = ctx.get_compiled_function(main);
    dbg!(before.elapsed());

    dbg!();
    dbg!(main.call(12));
}
