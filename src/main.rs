use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    // we can probably flip the traits so that we can let rust do the type inference?
    // TODO: I don't think we need to name the function
    let main = ctx.func::<(&&str, &&str), ()>("main", |(_s1, _s2)| {
        let func = (|x: i64| { println!("hello: {x}"); 0 }).into_host_fn();
        let x = Val::new(1i64);
        let y = Val::new(1i64);
        let mut tot = Var::new(x + y);
        tot += 1i64;
        tot *= 2i64;
        func.call(tot);
        // let double_f = (|s: &str, y: u32| {
        //     println!("s: {s}, y: {y}");
        // }).into_host_fn();
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call((&"helloooooo", &"balbal")));
    dbg!(before.elapsed());
}
