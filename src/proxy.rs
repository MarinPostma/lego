use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::{types::*, InstBuilder as _, MemFlags};
use cranelift::prelude::*;

use crate::func::{with_ctx, FnCtx};
use crate::types::{IntoVal, Val};


impl Proxy<u64> {
    fn load(&self, ctx: &mut FnCtx) -> Value {
        ctx.builder.ins().load(I64, MemFlags::new(), self.addr, self.offset)
    }

    pub fn get(&self) -> Val<u64> {
        with_ctx(|ctx| {
            let value = self.load(ctx);
            Val::new(value)
        })
    }
}

#[repr(transparent)]
pub struct ProxyMut<T>(Proxy<T>);

impl<T> ProxyMut<T> {
    #[doc(hidden)]
    pub fn new(addr: Value, offset: i32) -> Self {
        Self(Proxy::new(addr, offset))
    }
}

impl<T> Deref for ProxyMut<T> {
    type Target = Proxy<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ProxyMut<u64> {
    fn store(&mut self, ctx: &mut FnCtx, val: Value) {
        ctx.builder.ins().store(MemFlags::new(), val, self.addr, self.offset);
    }

    pub fn put(&mut self, val: impl IntoVal<u64>) {
        with_ctx(|ctx| {
            let val = val.into_val(ctx);
            self.store(ctx, val.value());
        });
    }
}

pub struct Proxy<T> {
    addr: Value,
    offset: i32,
    _pth: PhantomData<T>,
}

impl<T> Proxy<T> {
    #[doc(hidden)]
    pub fn addr(&self) -> Value {
        self.addr
    }

    #[doc(hidden)]
    pub fn offset(&self) -> i32 {
        self.offset
    }

    #[doc(hidden)]
    pub fn new(addr: Value, offset: i32) -> Self {
        Self { addr, offset, _pth: PhantomData }
    }
}
