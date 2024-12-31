use std::marker::PhantomData;

use cranelift::prelude::{Block, InstBuilder as _};

use crate::func::{with_ctx, FnCtx};

use super::{BlockRet, Branch, Cond};

pub struct If<O> {
    else_block: Block,
    merge_block: Block,
    is_built: bool,
    _output: PhantomData<O>,
}

impl<O: BlockRet> If<O> {
    #[doc(hidden)]
    #[must_use]
    pub fn new<C: Cond>(cond: C) -> Self {
        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<O>);

        let cond_val = cond.eval();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        Self {
            else_block,
            merge_block,
            is_built: false,
            _output: PhantomData,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn then<T>(mut self, then: Then<T>) -> Then2<O>
    where
        Then<T>: Branch<Output = O>,
    {
        let then_val = then.eval();

        with_ctx(|ctx| {
            O::jump_to(then_val, ctx, self.merge_block);

            ctx.builder().switch_to_block(self.else_block);
            ctx.builder().seal_block(self.else_block);
        });

        self.is_built = true;
        Then2 {
            merge_block: self.merge_block,
            is_built: false,
            _output: PhantomData,
        }
    }
}

pub struct Then2<O> {
    merge_block: Block,
    is_built: bool,
    _output: PhantomData<O>,
}

impl<O: BlockRet> Then2<O> {
    pub fn alt<E>(mut self, alt: Else<E>) -> O
    where
        Else<E>: Branch<Output = O>,
    {
        self.is_built = true;

        let else_val = alt.eval();

        with_ctx(|ctx| {
            O::jump_to(else_val, ctx, self.merge_block);

            let b = ctx.builder();
            b.switch_to_block(self.merge_block);
            b.seal_block(self.merge_block);
            O::read_from_ret(ctx, self.merge_block)
        })
    }
}

struct Never;

impl Branch for Else<Never> {
    type Output = ();

    fn eval(self) -> Self::Output { }
}

impl Then2<()> {
    pub fn finish(self) {
        self.alt(Else(Never))
    }
}

impl<O> Drop for Then2<O> {
    fn drop(&mut self) {
        assert!(self.is_built, "if must be built, of an else branch must be provided");
    }
}


impl<O> Drop for If<O> {
    fn drop(&mut self) {
        assert!(self.is_built, "missing if branch is invalid");
    }
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
    O: BlockRet,
{
    type Output = O;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}

impl<T, O> Branch for Else<T>
where
    T: FnOnce() -> O,
    O: BlockRet,
{
    type Output = O;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}
