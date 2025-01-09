use lego::ffi::Function;
use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let main = ctx.func::<usize, i32>(|_val| ControlFlow::Break(0.value()));

    // ctx.disas(main);

    let main = ctx.get_compiled_function(main);

    dbg!();
    dbg!(main.call(12));
}
