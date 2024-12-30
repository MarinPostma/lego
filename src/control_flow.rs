use std::marker::PhantomData;

use cranelift::prelude::InstBuilder;

use crate::{func::with_ctx, types::{ToJitPrimitive, Val}};

struct If<C>(pub C);
struct Then<T>(pub T);
struct Else<E>(pub E);

struct Cond<C, T, E = ()> {
    cond: If<C>,
    then: Then<T>,
    alt: Option<Else<E>>,
}

pub trait Branch {
    type Output;

    fn eval(self) -> Self::Output;
}

impl Branch for () {
    type Output = ();

    fn eval(self) -> Self::Output {
        todo!()
    }
}

impl<C> Branch for If<C>
    where C: FnOnce() -> Val<bool>
{
    type Output = Val<bool>;

    fn eval(self) -> Self::Output {
        (self)()
    }
}

impl<C, T, E, O> Cond<C, T, E> 
    where
        Then<T>: Branch<Output = Val<O>>,
        Else<E>: Branch<Output = Val<O>>,
        If<C>: Branch<Output = Val<bool>>,
        O: ToJitPrimitive,
        
{
    pub fn build(self) -> O {
        let (then_block, else_block, merge_block) = with_ctx(|ctx| {
            let b = ctx.builder();
            let then_block = b.create_block();
            let else_block = b.create_block();
            let merge_block = b.create_block();

            b.append_block_param(merge_block, O::ty());

            (then_block, else_block, merge_block)
        });

        let cond_val = self.cond.eval();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        let then_val = self.then.eval();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().jump(merge_block, &[then_val]);
        });

        let else_val = if let Some(else_branch) = self.alt {
            with_ctx(|ctx| {
                let b = ctx.builder();
                b.switch_to_block(else_block);
                b.seal_block(else_block);
            });

            let else_val = else_branch.eval();
        } else {
            with_ctx(|ctx| {
                ctx.builder().ins().iconst(0, O::ty());
            })
        };

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().jump(merge_block, &[else_val]);
            b.switch_to_block(merge_block);
            b.seal_block(merge_block);
            Val::new(b.block_params(merge_block)[0])
        })
    }
}
