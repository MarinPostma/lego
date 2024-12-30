use cranelift::prelude::{Block, InstBuilder};

use crate::types::{ToJitPrimitive, Val};
use crate::func::{with_ctx, FnCtx};

pub struct If<C, B>(pub C, pub B);
pub struct Then<T>(pub T);
pub struct Else<E>(pub E);

pub trait Conditional {
    type Output;

    fn build(self) -> Self::Output;
}

impl<C, T, E, O> Conditional for If<C, (Then<T>, Else<E>)>
where 
    C: Cond,
    Then<T>: Branch<Output = O>,
    Else<E>: Branch<Output = O>,
    O: BlockRet,
{
    type Output = O;

    fn build(self) -> Self::Output {
        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<O>);

        let cond_val = self.0.eval();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        let then_val = self.1.0.eval();

        with_ctx(|ctx| {
            O::jump_to(then_val, ctx, merge_block);

            ctx.builder().switch_to_block(else_block);
            ctx.builder().seal_block(else_block);
        });

        let else_val = self.1.1.eval();

        with_ctx(|ctx| {
            O::jump_to(else_val, ctx, merge_block);

            let b = ctx.builder();
            b.switch_to_block(merge_block);
            b.seal_block(merge_block);
            O::read_from_ret(ctx, merge_block)
        })
    }
}

/// Special case for single branch conditionals: Then must return `()`
impl<C, T> Conditional for If<C, Then<T>> 
    where
    C: Cond,
    Then<T>: Branch<Output = ()>
{
    type Output = ();

    fn build(self) -> Self::Output {
        If(self.0, (self.1, Else(EmptyElse))).build()
    }
}

fn make_cond_blocks<T: BlockRet>(ctx: &mut FnCtx) -> [Block; 3] {
    let b = ctx.builder();
    let then_block = b.create_block();
    let else_block = b.create_block();
    let merge_block = b.create_block();

    T::push_param_ty(ctx, merge_block);

    [then_block, else_block, merge_block]
}

pub trait Branch {
    type Output;

    fn eval(self) -> Self::Output;
}

struct EmptyElse;

impl Branch for Else<EmptyElse> {
    type Output = ();

    fn eval(self) -> Self::Output { }
}

impl<T, O> Branch for Then<T>
where
    T: FnOnce() -> Val<O>,
    O: ToJitPrimitive,
{
    type Output = Val<O>;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}

impl<T, O> Branch for Else<T>
where
    T: FnOnce() -> Val<O>,
    O: ToJitPrimitive,
{
    type Output = Val<O>;

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}

/// Something the returns a Val<bool> that can be used in a comparison
trait Cond {
    fn eval(self) -> Val<bool>;
}

impl<C> Cond for C
    where C: FnOnce() -> Val<bool>
{
    fn eval(self) -> Val<bool> {
        (self)()
    }
}

trait BlockRet {
    /// push param ty for the passed block
    fn push_param_ty(ctx: &mut FnCtx, block: Block);
    fn jump_to(self, ctx: &mut FnCtx, block: Block);
    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self;
}

impl BlockRet for () {
    fn push_param_ty(_ctx: &mut FnCtx, _block: Block) { }
    fn jump_to(self, ctx: &mut FnCtx, block: Block) { 
        ctx.builder().ins().jump(block, &[]);
    }

    fn read_from_ret(_ctx: &mut FnCtx, _block: Block) -> Self { }
}

impl<T: ToJitPrimitive> BlockRet for Val<T> {
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        ctx.builder().append_block_param(block, T::ty());
    }

    fn jump_to(self, ctx: &mut FnCtx, block: Block) {
        ctx.builder().ins().jump(block, &[self.value()]);
    }

    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self {
        Val::new(ctx.builder().block_params(block)[0])
    }
}

#[macro_export]
macro_rules! lego_if {
    (if ($cond:expr) { $then:expr } else { $else:expr }) => {
        {
            use $crate::control_flow::Conditional;

            $crate::control_flow::If(|| { $cond }, (
                $crate::control_flow::Then(|| { $then }),
                $crate::control_flow::Else(|| { $else }),
            )).build()
        }

    };
}
