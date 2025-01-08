use lego::{ffi::Function, prelude::*};

fn main() {
    let arg: i32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let main = ctx.func::<(&[i32], &[i32]), i32>(|(val, hello)| {
        let print_usize = (|val: usize| println!("{val}")).into_host_fn();
        let print= (|val: i32| println!("{val}")).into_host_fn();
        // right now we can't define closure within lego
        print_usize.fn_call(val.len());
        {
            let mut i = Var::new.fn_call((0usize,));
            {
                lego::prelude::do_while(|__ctx__|{
                    while __ctx__.cond(| |i.neq(&val.len())){
                        {
                            print.fn_call(val.get(i));
                            i += 1usize;
                        }
                    }lego::prelude::ControlFlow::<(), ()>::Break(())
                })
            }
};
        ControlFlow::Break(0.value())
    });

    // ctx.disas(main);

    let main = ctx.get_compiled_function(main);

    dbg!(main.call(([1, 2, 3, 4].as_slice(), &[1i32])));
}
