use std::marker::PhantomData;

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

pub trait Cond<B, R> {
    fn eval(&mut self) -> ControlFlow<B, R>;
}

impl<C, B, R, T, A> Cond<B, R> for If3<C, B, R, T, A>
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
