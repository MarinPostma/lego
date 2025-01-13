use std::ops::Range;

use cranelift::prelude::InstBuilder;

use crate::cmp::Compare;
use crate::control_flow::while_loop::do_while2;
use crate::control_flow::BlockRet;
use crate::func::with_ctx;
use crate::prelude::Primitive;
use crate::val::{AsVal, Val};
use crate::var::Var;

pub trait JIterator {
    type Item;

    fn next(&mut self) -> (Val<bool>, Self::Item);

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

    fn for_each<F>(mut self, mut f: F)
    where
        F: FnMut(Self::Item),
        Self::Item: BlockRet,
        Self: Sized,
    {
        let [header, body, exit] = with_ctx(|ctx| {
            let [header, body, exit] = ctx.create_blocks();
            ctx.builder().ins().jump(header, &[]);
            Self::Item::push_param_ty(ctx, body);
            ctx.builder().switch_to_block(header);
            [header, body, exit]
        });

        let (has_it, it) = self.next();

        let it = with_ctx(|ctx| {
            let mut then_params = Vec::new();
            it.to_block_values(&mut then_params);
            ctx.builder().ins().brif(
                has_it.value,
                body,
                &then_params,
                exit,
                &[]);

            ctx.builder().switch_to_block(body);
            ctx.builder().seal_block(body);
            Self::Item::read_from_ret(&mut ctx.builder().block_params(body).iter().copied())
        });

        f(it);

        with_ctx(|ctx| {
            ctx.builder().ins().jump(header, &[]);

            ctx.builder().seal_block(header);
            ctx.builder().switch_to_block(exit);
            ctx.builder().seal_block(exit);
        });
    }

    fn fold<F, B>(mut self, init: B, f: F) -> B
    where
        B: AsVal + BlockRet,
        F: FnOnce(B, Self::Item) -> B,
        Self: Sized,
    {
        let mut has_more = Var::new(true);
        do_while2(init, |_init| {
            (has_more.value(), |init| {
                let (more, val) = self.next();
                has_more.assign(more);
                f(init, val)
            })
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

    fn next(&mut self) -> (Val<bool>, Self::Item) {
        let (has_more, val) = self.inner.next();
        (has_more, (self.f)(val))
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

impl<Idx> JIterator for RangeJiter<Idx>
where
    Val<Idx>: Compare,
    Var<Idx>: Step,
{
    type Item = Val<Idx>;

    fn next(&mut self) -> (Val<bool>, Self::Item) {
        let ret = (self.start.value().neq(self.end), self.start.value());
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

    fn next(&mut self) -> (Val<bool>, Self::Item) {
        let [header_block, body_block, exit_block] = with_ctx(|ctx| {
            let [header_block, body_block, exit_block] = ctx.create_blocks();
            <(Val<bool>, Self::Item) as BlockRet>::push_param_ty(ctx, exit_block);
            Self::Item::push_param_ty(ctx, body_block);
            ctx.builder().ins().jump(header_block, &[]);
            ctx.builder().switch_to_block(header_block);
            [header_block, body_block, exit_block]
        });

        let (has_it, it) = self.inner.next();

        let it = with_ctx(|ctx| {
            let mut then_params = Vec::new();
            it.to_block_values(&mut then_params);
            let mut else_params = Vec::new();
            (has_it, it).to_block_values(&mut else_params);
            ctx.builder().ins().brif(
                has_it.value(),
                body_block,
                &then_params,
                exit_block,
                &else_params
            );

            ctx.builder().switch_to_block(body_block);
            ctx.builder().seal_block(body_block);
            Self::Item::read_from_ret(&mut ctx.builder.block_params(body_block).iter().copied())
        });

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
            <_ as BlockRet>::read_from_ret(&mut ctx.builder.block_params(exit_block).iter().copied())
        })
    }
}
