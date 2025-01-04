use lego::prelude::*;

fn main() {
    let arg: i32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let main = ctx.func::<i32, i32>(|val| {
        let say_hello_odd = (|val: i32| { println!("hello!even: {val}")}).into_host_fn();
        let say_hello_even = (|val: i32| { println!("hello!odd: {val}")}).into_host_fn();
        let say_hello_winner = (|val: i32| { println!("this is a winner: {val}")}).into_host_fn();
        ControlFlow::Ret(
            lego!({
                for _ in 0..2 { // <- do 2 times, this can be thought of as inlining, or loop unrolling
                    let mut i = Var::new(0);
                    while i.neq(&10) { // <- emit while
                        if i.eq(&Val::new(arg)) { // <- this is dynamic! got that `arg` at runtime
                            say_hello_winner.call(i);
                        } else if (i % 2).eq(&0) { // <- emit if/else
                            say_hello_even.call(i);
                        } else {
                            say_hello_odd.call(i);
                        }
                        i += 1;
                    }
                }

                val + 1
            })
        )
    });

    let main = ctx.get_compiled_function(main);

    dbg!();
    dbg!(main.call(41));
}
