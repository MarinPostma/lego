use cranelift::prelude::{Block, InstBuilder as _};

use crate::{func::{with_ctx, FnCtx}, types::{ToJitPrimitive, Val}};

use super::{BlockRet, Cond};

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
    let [then_block, else_block, merge_block] = ctx.create_blocks();
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


#[macro_export]
macro_rules! lego_if {
    (if ($cond:expr) { $($then:tt)* } else { $($else:expr)* }) => {
        {
            use $crate::control_flow::Conditional;

            $crate::control_flow::If(|| { $cond }, (
                $crate::control_flow::Then(|| { $($then)* }),
                $crate::control_flow::Else(|| { $($else)* }),
            )).build()
        }
    };
    (if ($cond:expr) { $then:expr }) => {
        {
            use $crate::control_flow::Conditional;

            $crate::control_flow::If(|| { $cond }, 
                $crate::control_flow::Then(|| { $then }),
            ).build()
        }
    };
}
