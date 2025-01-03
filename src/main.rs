use std::time::Instant;

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();

    let main = ctx.func::<i32, i32>(|val| {
        ControlFlow::Ret({
            {
                #[allow(unreachable_code)]
                let r = lego::prelude::If3::new(
                    || val.eq(&42i32),
                    |__ctx__| {
                        lego::prelude::ControlFlow::Break({
                            match __ctx__.ret(Val::new(12i32)) {
                                lego::prelude::ControlFlow::Continue => unreachable!(),
                                lego::prelude::ControlFlow::Break(v) => {
                                    return lego::prelude::ControlFlow::Break(v)
                                }
                                lego::prelude::ControlFlow::Ret(v) => {
                                    return lego::prelude::ControlFlow::Ret(v)
                                }
                                lego::prelude::ControlFlow::Preempt => {
                                    return lego::prelude::ControlFlow::Preempt
                                }
                            }
                        })
                    },
                    |__ctx__| lego::prelude::ControlFlow::Break(()),
                )
                .eval();
                match r {
                    lego::prelude::ControlFlow::Continue => {
                        return lego::prelude::ControlFlow::Continue
                    }
                    lego::prelude::ControlFlow::Break(v) => v,
                    lego::prelude::ControlFlow::Ret(v) => {
                        return lego::prelude::ControlFlow::Ret(v)
                    }
                    lego::prelude::ControlFlow::Preempt => {
                        return lego::prelude::ControlFlow::Preempt
                    }
                }
            }
            val + 1i32
        })
    });
    dbg!(before.elapsed());

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    dbg!(main.call(43));
    dbg!(before.elapsed());
}
