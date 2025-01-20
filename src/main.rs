use std::time::Instant;

use lego::ffi::Function;
use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    let main = ctx.func::<&[i32], i32>(|items| {
        let print = (|v: i32| println!("val: {v}")).into_host_fn();
        // it1
        //     .into_jiter()
        //     // .filter(|it| {
        //     //     (it.deref() % 2).eq(0u64.value())
        //     // })
        //     // .map(|it| it.deref() + 1)
        //     .for_each(|it| {
        //         print.call(it.deref());
        //     });

    let sum = items
        .into_jiter()
        .map(|it| it.deref() + 1i32)
        .fold(0i32.value(), |acc, it| {
            acc + it
        });

    sum
    });

    let main = ctx.get_compiled_function(main);

    let items = &[2, 2, 2, 1];
    dbg!(main.call(items));
}
