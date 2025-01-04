use core::panic;
use std::any::Any;

use cranelift::prelude::{Block, InstBuilder};

use crate::{func::with_ctx, val::Val};

use super::if_then_else2::ControlFlow;

pub trait Cond<R, Test> {
    /// initialize the condition, returning the state
    fn init() -> Box<dyn CondState<R>>;
    fn eval(&mut self, state: &mut dyn CondState<R>) -> bool;
}

impl<R> CondState<R> for () {
    fn ret(&mut self, r: R) -> ControlFlow<(), R> {
        ControlFlow::Ret(r)
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<R, F> Cond<R, bool> for F
where
    F: FnMut() -> bool,
{
    fn init() -> Box<dyn CondState<R>> {
        Box::new(())
    }

    fn eval(&mut self, state: &mut dyn CondState<R>) -> bool {
        state.as_mut_any().downcast_mut::<()>().expect("loop is not well typed");
        (self)()
    }
}

struct GenState {
    has_body: bool,
    body_block: Block,
    exit_block: Block,
    header_block: Block,
    finalized: bool,
}

impl<R> CondState<R> for GenState {
    fn ret(&mut self, _r: R) -> ControlFlow<(), R> {
        // emit return
        ControlFlow::Preempt
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait CondState<R> {
    fn ret(&mut self, r: R) -> ControlFlow<(), R>;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

impl<R, F> Cond<R, Val<bool>> for F
where
    F: FnMut() -> Val<bool>,
{
    fn init() -> Box<dyn CondState<R>> {
        let [header_block, body_block, exit_block] = with_ctx(|ctx| {
            let [header_block, body_block, exit_block] = ctx.create_blocks();
    
            let builder = ctx.builder();
            builder.ins().jump(header_block, &[]);
            builder.switch_to_block(header_block);
            [header_block, body_block, exit_block]
        });

        Box::new(GenState { has_body: false, body_block, exit_block, finalized: false, header_block })
    }

    fn eval(&mut self, state: &mut dyn CondState<R>) -> bool {
        let state = state.as_mut_any().downcast_mut::<GenState>().unwrap();
        if state.has_body {
            if !state.finalized {
                with_ctx(|ctx| {
                    let builder = ctx.builder();
                    builder.ins().jump(state.header_block, &[]);
                    builder.switch_to_block(state.exit_block);
                    builder.seal_block(state.header_block);
                    builder.seal_block(state.exit_block);
                });

                state.finalized = true;
            } else {
                panic!("loop already finalized")
            }

            false
        } else {
            let header_val = (self)();

            with_ctx(|ctx| {
                let builder = ctx.builder();
                builder.ins().brif(header_val.value(), state.body_block, &[], state.exit_block, &[]);
                builder.switch_to_block(state.body_block);
                builder.seal_block(state.body_block);
            });
            state.has_body = true;
            true
        }
    }
}

pub struct WhileCtx<R> {
    state: Option<Box<dyn CondState<R>>>,
}

impl<R> WhileCtx<R> {
    fn state(&mut self) -> &mut dyn CondState<R> {
        let state = self.state.as_mut().unwrap();
        &mut **state
    }

    pub fn cond<Test, C: Cond<R, Test>>(&mut self, mut cond: C) -> bool {
        if self.state.is_none() {
            self.state = Some(C::init());
        }

        cond.eval(self.state())
    }

    pub fn ret(&mut self, r: R) -> ControlFlow<(), R> {
        self.state().ret(r)
    }
}


pub fn do_while<R>(
    f: impl FnOnce(&mut WhileCtx<R>) -> ControlFlow<(), R>,
) -> ControlFlow<(), R> {
    let mut ctx = WhileCtx {
        state: None,
    };

    f(&mut ctx)
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn while_example() {
        fn while_example() -> u64 {
            let mut i = 0;
            let ret = do_while(|ctx| {
                while ctx.cond(|| i != 10) {
                    // that's the body verbatim
                    println!("hello");
                    if i == 5 {
                        match ctx.ret(32) {
                            ControlFlow::Break(x) => return ControlFlow::Break(x),
                            ControlFlow::Ret(x) => return ControlFlow::Ret(x),
                            ControlFlow::Continue => todo!(),
                            ControlFlow::Preempt => (),
                        }
                    }
                    i += 1;
                }
                ControlFlow::Break(())
            });

            match ret {
                ControlFlow::Break(()) => (),
                ControlFlow::Ret(v) => return v,
                ControlFlow::Continue => todo!(),
                ControlFlow::Preempt => todo!(),
            }

            12
        }

        dbg!(while_example());
    }
}
