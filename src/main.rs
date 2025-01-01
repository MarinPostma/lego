use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();
    // we can probably flip the traits so that we can let rust do the type inference?
    // TODO: I don't think we need to name the function
    let main = ctx.func::<&str, u32>("main", |s| {
        let x = Val::new(12u64);
        let y = Val::new(13u64);
        let func = (|hello: &str| { println!("hello: {hello}"); 0 }).into_host_fn();
        func.call(s.get_ref())
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call("helloooooo"));
    dbg!(before.elapsed());
}
