use lego::prelude::*;

fn main() {
    let arg: i32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    /// this function generates code depending on the T type param: we can generate specialized
    /// implementation of pow on the fly, if a very readable way
    fn pow<T>(val: T, to: usize) -> impl AsVal<Ty = T::Ty>
    where
        T: AsVal,
        T::Ty: IntMul,
    {
        let v = val.value();
        let mut x = Var::new(val);
        for _ in 1..to {
            x *= v;
        }

        x
    }

    let main = ctx.func::<i32, i32>(|val| {
        // right now we can't define closure within lego
        let say_hello_odd = (|val: i32| { println!("hello!odd: {val}")}).into_host_fn();
        let say_hello_even = (|val: i32| { println!("hello!even: {val}")}).into_host_fn();
        let say_hello_winner = (|val: i32| { println!("this is a winner: {val}")}).into_host_fn();
        lego!({
            for _ in 0..2 { // <- do 2 times, this can be thought of as inlining, or loop
                // unrolling, this is partially evaluated
                let mut i = Var::new(0);
                while i.neq(&10) { // <- emit while
                    if i.eq(&Val::new(arg)) { // <- this is dynamic! got that `arg` at runtime
                        if arg == 5 { // <- this is partially evaluated during function building
                            return val + 0;
                        } else {
                            say_hello_winner(i);
                        }
                    } else if (i % 2).eq(&0) { // <- emit if/else
                        say_hello_even(i);
                    } else {
                        say_hello_odd(i);
                    }
                    i += 1;
                }
            }
            
            say_hello_odd(val);
            // loop unwinded implementation of pow3
            pow(val, 3).value()
        })
    });

    // ctx.disas(main);

    let main = ctx.get_compiled_function(main);

    dbg!();
    dbg!(main.fn_call(2));
}
