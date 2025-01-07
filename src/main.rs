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

    let main = ctx.func::<&&[u8], i32>(|val| {
        // right now we can't define closure within lego
        ControlFlow::Break(1.value())
    });

    // ctx.disas(main);

    let main = ctx.get_compiled_function(main);

    // dbg!();
    // dbg!(main.fn_call(&&[]));
}
