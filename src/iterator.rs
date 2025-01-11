use std::ops::Range;

use crate::control_flow::while_loop::do_while2;
use crate::control_flow::BlockRet;
use crate::val::{AsVal, Val};
use crate::prelude::Primitive;
use crate::cmp::Compare;
use crate::var::Var;

pub trait JIterator {
    type Item;

    fn next(&mut self) -> (Val<bool>, Self::Item);

    fn map<F, B>(self, f: F) -> Map<Self, F> 
    where 
        F: FnMut(Self::Item) -> B,
        Self: Sized,
    {
        Map {
            inner: self,
            f,
        }
    }

    fn filter<F>(self, f: F) -> Filter<Self, F>
    where
        F: FnMut(&Self::Item) -> Val<bool>,
        Self: Sized,
    {
        Filter {
            inner: self,
            f,
        }
    }

    fn for_each<F>(mut self, mut f: F)
    where
        F: FnMut(Self::Item),
        Self: Sized,
    {
        let mut has_more = Var::new(true);
        do_while2((), || {
            (has_more.value(), |_| {
                let (more, val) = self.next();
                has_more.assign(more);
                f(val);
            })
        })
    }

    fn fold<F, B>(mut self, init: B, f: F) -> B
    where
        B: AsVal + BlockRet,
        F: FnOnce(B, Self::Item) -> B,
        Self: Sized,
    {
        let mut has_more = Var::new(true);
        do_while2(init, || {
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

impl<Idx> JIterator for RangeJiter<Idx>
where 
    Val<Idx>: Compare,
    Var<Idx>: Step,
{
    type Item = Val<Idx>;

    fn next(&mut self) -> (Val<bool>, Self::Item) {
        let current = self.start.value();
        self.start.step();
        (self.start.value().neq(self.end), current)
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
        let mut has_more = Var::new(true);
        let mut it = None;
        let mut cont = Var::new(true);
        do_while2((), || (cont.value(), |_| {
            let (more, val) = self.inner.next();
            let take = (self.f)(&val);
            it = Some(val);
            has_more.assign(more);
            cont.assign(more & take);
        }));

        (has_more.value(), it.unwrap())
    }
}
