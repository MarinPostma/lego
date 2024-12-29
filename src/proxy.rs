use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::{types::*, InstBuilder as _, MemFlags};
use cranelift::prelude::*;

use crate::func::{host_fn, with_ctx, FnCtx, IntoParams, Param};
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

impl<T> IntoParams<&mut T> for Proxy<T> {
    fn params(&self, _ctx: &mut FnCtx, out: &mut Vec<Value>) {
        out.push(self.addr());
    }
}

impl<K, V> Proxy<HashMap<K, V>>
{
    pub fn insert<'a>(&'a mut self, k: impl IntoParams<K>, v: impl IntoParams<V>)
    where 
        K: Hash + Eq + Param + 'a,
        V: Param + 'a,
    {
        extern "C" fn insert<'b, K, V>(map: &'b mut HashMap<K, V>, k: K, v: V)
        where K: Hash + Eq + 'b,
            V: 'b,
        {
            map.insert(k, v);
        }

        let f = host_fn(insert::<K, V> as extern "C" fn (&'a mut HashMap<K, V>, k: K, v: V));

        f.call((self, k, v));
    }
}
