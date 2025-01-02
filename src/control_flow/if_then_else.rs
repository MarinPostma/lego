use cranelift::prelude::{Block, InstBuilder as _};

use crate::{func::{with_ctx, FnCtx}, val::{AsVal, Val}};

use super::{BlockRet, Branch};

pub trait Cond<T, E, Test> {
    type Output;

    fn eval(&mut self, then: T, alt: E) -> Self::Output;
}

impl<F, T, E, O> Cond<T, E, bool> for F
    where
        F: FnMut() -> bool,
        T: Branch<Output = O>,
        E: Branch<Output = O>,
{
    type Output = O;

    fn eval(&mut self, then: T, alt: E) -> Self::Output {
        if (self)() {
            then.eval()
        } else {
            alt.eval()
        }
    }
}

impl<F, T, E, O> Cond<T, E, Val<bool>> for F
    where
        F: FnMut() -> Val<bool>,
        T: Branch,
        E: Branch,
        T::Output: AsVal<Ty = O>,
        E::Output: AsVal<Ty = O>,
        Val<O>: BlockRet,
{
    type Output = Val<O>;

    fn eval(&mut self, then: T, alt: E) -> Self::Output {
        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<Val<O>>);

        let cond_val = (self)();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        let then_val = then.eval();

        with_ctx(|ctx| {
            let then_val = then_val.as_val(ctx);
            <Val<O>>::jump_to(then_val, ctx, merge_block);

            ctx.builder().switch_to_block(else_block);
            ctx.builder().seal_block(else_block);
        });

        let else_val = alt.eval();

        with_ctx(|ctx| {
            let else_val = else_val.as_val(ctx);
            <Val<O>>::jump_to(else_val, ctx, merge_block);

            let b = ctx.builder();
            b.switch_to_block(merge_block);
            b.seal_block(merge_block);
            <Val<O>>::read_from_ret(ctx, merge_block)
        })

    }
}

pub struct Never;

impl Branch for Else<Never> {
    type Output = ();

    fn eval(self) -> Self::Output { }
}

pub struct Then<T>(pub T);
pub struct Else<E>(pub E);

fn make_cond_blocks<T: BlockRet>(ctx: &mut FnCtx) -> [Block; 3] {
    let [then_block, else_block, merge_block] = ctx.create_blocks();
    T::push_param_ty(ctx, merge_block);
    [then_block, else_block, merge_block]
}

impl<T, O> Branch for Then<T>
where
    T: FnOnce() -> O,
{
    type Output = O;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}

impl<T, O> Branch for Else<T>
where
    T: FnOnce() -> O,
{
    type Output = O;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}
