use std::ops::Range;

use cranelift::prelude::InstBuilder;

use crate::cmp::Compare;
use crate::control_flow::BlockRet;
use crate::func::with_ctx;
use crate::prelude::Primitive;
use crate::val::{AsVal, Val};
use crate::var::Var;

pub trait JIterator {
    type Item;

    fn next(&mut self) -> (Val<bool>, impl FnOnce() -> Self::Item);

    fn map<F, B>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> B,
        Self: Sized,
    {
        Map { inner: self, f }
    }

    fn filter<F>(self, f: F) -> Filter<Self, F>
    where
        F: FnMut(&Self::Item) -> Val<bool>,
        Self: Sized,
    {
        Filter { inner: self, f }
    }

    fn for_each<F>(self, f: F)
    where
        F: FnOnce(Self::Item),
        Self::Item: BlockRet,
        Self: Sized,
    {
        self.fold((), |_, it| {
            f(it)
        })
    }

    fn fold<F, B>(mut self, init: B, f: F) -> B
    where
        B: BlockRet,
        F: FnOnce(B, Self::Item) -> B,
        Self: Sized,
        Self::Item: BlockRet,
    {
        let [header, body, exit] = with_ctx(|ctx| {
            let [header, body, exit] = ctx.create_blocks();
            B::push_param_ty(ctx, header);
            B::push_param_ty(ctx, exit);
            <B>::push_param_ty(ctx, body);
            let mut params = Vec::new();
            init.to_block_values(&mut params);
            ctx.builder().ins().jump(header, &params);
            ctx.builder().switch_to_block(header);
            [header, body, exit]
        });

        let (has_it, it) = self.next();

        let acc = with_ctx(|ctx| {
            let mut then_params = Vec::new();
            let init = B::read_from_ret(&mut ctx.builder().block_params(header).iter().copied());
            let mut params = Vec::new();
            init.to_block_values(&mut params);
            init.to_block_values(&mut then_params);
            ctx.builder().ins().brif(
                has_it.value,
                body,
                &then_params,
                exit,
                &params);

            ctx.builder().switch_to_block(body);
            ctx.builder().seal_block(body);
            <B>::read_from_ret(&mut ctx.builder().block_params(body).iter().copied())
        });

        let acc = f(acc, it());

        with_ctx(|ctx| {
            let mut params = Vec::new();
            acc.to_block_values(&mut params);
            ctx.builder().ins().jump(header, &params);

            ctx.builder().seal_block(header);
            ctx.builder().switch_to_block(exit);
            ctx.builder().seal_block(exit);
            B::read_from_ret(&mut ctx.builder().block_params(exit).iter().copied())
        })
    }
}

pub struct Map<T, F> {
    inner: T,
    f: F,
}

impl<T, F, B> JIterator for Map<T, F>
where
    T: JIterator,
    F: FnMut(T::Item) -> B,
    B: BlockRet,
{
    type Item = B;

    fn next(&mut self) -> (Val<bool>, impl FnOnce() -> Self::Item) {
        let (has_it, val) = self.inner.next();
        (has_it, || (self.f)(val()))
    }
}

pub trait IntoJiter {
    type Iter: JIterator<Item = Self::Item>;
    type Item;

    fn into_jiter(self) -> Self::Iter;
}

pub struct RangeJiter<Idx> {
    start: Var<Idx>,
    end: Val<Idx>,
}

trait Step {
    fn step(&mut self);
}

impl Step for Var<usize> {
    fn step(&mut self) {
        *self += 1usize;
    }
}

impl Step for Var<u64> {
    fn step(&mut self) {
        *self += 1u64;
    }
}

impl Step for Var<i32> {
    fn step(&mut self) {
        *self += 1i32;
    }
}

impl<Idx> JIterator for RangeJiter<Idx>
where
    Val<Idx>: Compare,
    Var<Idx>: Step,
{
    type Item = Val<Idx>;

    fn next(&mut self) -> (Val<bool>, impl FnOnce() -> Self::Item) {
        let val = self.start.value();
        let ret = (self.start.value().neq(self.end), move || val);
        self.start.step();
        ret
    }
}

impl<Idx> IntoJiter for Range<Idx>
where
    Val<Idx>: Compare,
    Var<Idx>: Step,
    Idx: Primitive + AsVal<Ty = Idx>,
{
    type Iter = RangeJiter<Idx>;
    type Item = Val<Idx>;

    fn into_jiter(self) -> Self::Iter {
        RangeJiter {
            start: Var::new(self.start),
            end: Val::new(self.end),
        }
    }
}

pub struct Filter<T, F> {
    inner: T,
    f: F,
}

impl<T, F> JIterator for Filter<T, F>
where
    T: JIterator,
    F: FnMut(&T::Item) -> Val<bool>,
    T::Item: BlockRet,
{
    type Item = T::Item;

    fn next(&mut self) -> (Val<bool>, impl FnOnce() -> Self::Item) {
        let [header_block, body_block, exit_block] = with_ctx(|ctx| {
            let [header_block, body_block, exit_block] = ctx.create_blocks();
            <(Val<bool>, Self::Item) as BlockRet>::push_param_ty(ctx, exit_block);
            // Self::Item::push_param_ty(ctx, body_block);
            ctx.builder().ins().jump(header_block, &[]);
            ctx.builder().switch_to_block(header_block);
            [header_block, body_block, exit_block]
        });
        
        let (has_it, it) = self.inner.next();
        
        with_ctx(|ctx| {
            let then_params = Vec::new();
            let mut else_params = Vec::new();
            has_it.to_block_values(&mut else_params);
            Self::Item::null(ctx, &mut else_params);

            ctx.builder().ins().brif(
                has_it.value(),
                body_block,
                &then_params,
                exit_block,
                &else_params
            );
        
            ctx.builder().switch_to_block(body_block);
            ctx.builder().seal_block(body_block);
        });
        
        let it = it();
        let take = (self.f)(&it);
        
        with_ctx(|ctx| {
            let mut then_params = Vec::new();
            (has_it, it).to_block_values(&mut then_params);
            ctx.builder().ins().brif(
                take.value(),
                exit_block,
                &then_params,
                header_block,
                &[],
            );
        
            ctx.builder().seal_block(header_block);
            ctx.builder().switch_to_block(exit_block);
            ctx.builder().seal_block(exit_block);
            let (has_it, it) =  <_ as BlockRet>::read_from_ret(&mut ctx.builder.block_params(exit_block).iter().copied());
            (has_it, || it)
        })
    }
}
