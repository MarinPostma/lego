use std::time::Instant;

use lego::ffi::Function;
use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    let main = ctx.func::<(&[u64], &[u64]), i32>(|(it1, _it2)| {
        let print = (|v: u64| println!("val: {v}")).into_host_fn();
        it1
            .into_jiter()
            // .filter(|it| {
            //     (it.deref() % 2).eq(0u64.value())
            // })
            // .map(|it| it.deref() + 1)
            .for_each(|it| {
                print.call(it.deref());
            });

        let sum = (1..5)
            .into_jiter()
            // .filter(|it| {
            //     (it.deref() % 2).eq(0u64.value())
            // })
            // .map(|it| it.deref() + 1)
            .fold(0u64.value(), |acc, it| {
                print.call(it);
                acc + it
            });

        print.call(sum);

        0.value()
    });
    let main = ctx.get_compiled_function(main);
    dbg!(before.elapsed());

    let items = &[1, 2, 3, 4];
    dbg!(main.call((items, items)));
}
