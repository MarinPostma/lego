use std::marker::PhantomData;

use cranelift::prelude::{Block, InstBuilder as _};

use crate::{func::{with_ctx, FnCtx, FromFuncRet, Results}, val::{AsVal, Val}};

use super::BlockRet;

#[derive(Debug)]
pub enum ControlFlow<B, R> {
    Break(B),
    Ret(R),
    Continue,
}

pub struct If3<
    C,
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
        C,
        B,
        R,
        T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
        A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    > If3<C, B, R, T, A>
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

pub trait Cond<B, R, Ty> {
    fn eval(&mut self) -> ControlFlow<B, R>;
}

impl<C, B, R, T, A> Cond<B, R, bool> for If3<C, B, R, T, A>
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

impl<C, B, R, T, A> Cond<B, R, Val<bool>> for If3<C, B, R, T, A>
where
    C: FnMut() -> Val<bool>,
    T: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    A: FnMut(&dyn Ctx<B, R>) -> ControlFlow<B, R>,
    B: AsVal,
    B::Ty: BlockRet,
    R: FromFuncRet,
{
    fn eval(&mut self) -> ControlFlow<B, R> {

        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<B::Ty>);

        let cond_val = (self.cond)();

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        struct C;
        impl<B, R> Ctx<B, R> for C
            where R: FromFuncRet
        {
            fn ret(&self, r: R) -> ControlFlow<B, R> {
                with_ctx(|ctx| {
                });
                todo!()
            }
        }

        todo!()
    }
}

        // let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<Val<O>>);
        //
        // let cond_val = (self)();
        //
        // with_ctx(|ctx| {
        //     let b = ctx.builder();
        //     b.ins().brif(cond_val.value(), then_block, &[], else_block, &[]);
        //     b.switch_to_block(then_block);
        //     b.seal_block(then_block);
        // });
        //
        // let then_val = then.eval();
        //
        // with_ctx(|ctx| {
        //     let then_val = then_val.as_val(ctx);
        //     <Val<O>>::jump_to(then_val, ctx, merge_block);
        //
        //     ctx.builder().switch_to_block(else_block);
        //     ctx.builder().seal_block(else_block);
        // });
        //
        // let else_val = alt.eval();
        //
        // with_ctx(|ctx| {
        //     let else_val = else_val.as_val(ctx);
        //     <Val<O>>::jump_to(else_val, ctx, merge_block);
        //
        //     let b = ctx.builder();
        //     b.switch_to_block(merge_block);
        //     b.seal_block(merge_block);
        //     <Val<O>>::read_from_ret(ctx, merge_block)
        // })
        //
