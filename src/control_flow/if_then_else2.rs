use std::marker::PhantomData;

use cranelift::prelude::{Block, InstBuilder as _};

use crate::func::{with_ctx, FnCtx, FuncRet};
use crate::val::{Val};

use super::BlockRet;

#[derive(Debug)]
pub enum ControlFlow<B, R> {
    Break(B),
    Ret(R),
    Continue,
    /// the control flow was preempted by branch. This happens when the branch returns instead of
    /// yielding a value, when generating code. In this case, no jump to the merge block is
    /// emitted
    Preempt,
}

pub struct If3<
    C: FnMut() -> Test,
    Test,
    B,
    R,
    T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
> {
    cond: C,
    then: T,
    alt: A,
    _pth: PhantomData<(B, R)>,
}

impl<
        C: FnMut() -> Test,
        Test,
        B,
        R,
        T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
        A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    > If3<C, Test, B, R, T, A>
{
    pub fn new(cond: C, then: T, alt: A) -> Self {
        Self {
            cond,
            then,
            alt,
            _pth: PhantomData,
        }
    }
}

pub trait Ctx<B, R> {
    fn ret(&self, r: R) -> ControlFlow<B, R>;
}

pub trait Cond<B, R> {
    fn eval(&mut self) -> ControlFlow<B, R>;
}

impl<C, B, R, T, A> Cond<B, R> for If3<C, bool, B, R, T, A>
where
    C: FnMut() -> bool,
    T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
{
    fn eval(&mut self) -> ControlFlow<B, R> {
        struct C;
        impl<B, R> Ctx<B, R> for C {
            fn ret(&self, r: R) -> ControlFlow<B, R> {
                ControlFlow::Ret(r)
            }
        }

        if (self.cond)() {
            (self.then)(&mut C)
        } else {
            (self.alt)(&mut C)
        }
    }
}

fn make_cond_blocks<T: BlockRet>(ctx: &mut FnCtx) -> [Block; 3] {
    let [then_block, else_block, merge_block] = ctx.create_blocks();
    T::push_param_ty(ctx, merge_block);
    [then_block, else_block, merge_block]
}

impl<C, B, R, T, A> Cond<B, R> for If3<C, Val<bool>, B, R, T, A>
where
    C: FnMut() -> Val<bool>,
    T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    B: BlockRet,
    R: FuncRet,
{
    fn eval(&mut self) -> ControlFlow<B, R> {

        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<B>);

        let cond_val = (self.cond)();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        struct C;
        impl<B, R> Ctx<B, R> for C
            where R: FuncRet
        {
            fn ret(&self, r: R) -> ControlFlow<B, R> {
                with_ctx(|ctx| {
                    r.return_(ctx);
                });
                ControlFlow::Preempt
            }
        }

        let then_val = (self.then)(&C);


        let then_flow = with_ctx(|ctx| {
            let flow = match then_val {
                ControlFlow::Break(val) => {
                    // let then_val = val.as_val(ctx);
                    B::jump_to(val, ctx, merge_block);
                    ControlFlow::<(), R>::Break(())
                },
                ControlFlow::Ret(_) => todo!(),
                ControlFlow::Continue => todo!(),
                ControlFlow::Preempt => ControlFlow::Preempt,
            };

            ctx.builder().switch_to_block(else_block);
            ctx.builder().seal_block(else_block);
            flow
        });

        let else_val = (self.alt)(&C);
        
        with_ctx(|ctx| {
            let else_flow = match else_val {
                ControlFlow::Break(else_val) => {
                    B::jump_to(else_val, ctx, merge_block);
                    ControlFlow::<_, R>::Break(())
                },
                ControlFlow::Ret(_) => todo!(),
                ControlFlow::Continue => todo!(),
                ControlFlow::Preempt => ControlFlow::Preempt,
            };
        
            let b = ctx.builder();
            b.switch_to_block(merge_block);
            b.seal_block(merge_block);

            match (then_flow, else_flow) {
                // If wither branch can return a value, then we break that value
                (ControlFlow::Break(_), _) | (_, ControlFlow::Break(_)) => {
                    ControlFlow::Break(B::read_from_ret(ctx, merge_block))
                }
                (ControlFlow::Ret(_), _) | (_, ControlFlow::Ret(_)) => unreachable!(),
                (ControlFlow::Preempt, _) | (_, ControlFlow::Preempt) => ControlFlow::Preempt,
                (ControlFlow::Continue, ControlFlow::Continue) => todo!(),
            }
        })
    }
}
