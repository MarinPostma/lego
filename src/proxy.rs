use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::{types::*, InstBuilder as _, MemFlags};
use cranelift::prelude::*;
use cranelift_codegen::ir::SigRef;
use cranelift_module::Module;

use crate::func::{host_fn, with_ctx, FnCtx, IntoParams, Param};
use crate::types::{IntoVal, ToJitPrimitive, Val};


impl<T: ToJitPrimitive> Proxy<T> {
    fn load(&self, ctx: &mut FnCtx) -> Value {
        ctx.builder.ins().load(T::ty(), MemFlags::new(), self.addr, self.offset)
    }

    pub fn get(&self) -> Val<T> {
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

    fn get_mut(&mut self) -> RefMut<T> {
        RefMut {
            proxy: self.0.get_ref(),
        }
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

    pub fn get_ref(&self) -> Ref<T> {
        Ref{ proxy: self }
    }
}

pub struct Ref<'a, T> {
    proxy: &'a Proxy<T>
}

pub struct RefMut<'a, T> {
    proxy: Ref<'a, T>,
}

impl<'a, T> IntoParams for RefMut<'a, T> {
    type Input = &'a mut T;

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        self.proxy.params(ctx, out)
    }
}

impl<'a, T> IntoParams for Ref<'a, T> {
    type Input = &'a T;

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        let addr =if self.proxy.offset() != 0 {
            ctx.builder().ins().iadd_imm(self.proxy.addr(), self.proxy.offset() as i64)
        } else {
            self.proxy.addr()
        };
        out.push(addr);
    }
}

impl<K, V> ProxyMut<HashMap<K, V>>
{
    pub fn insert<'a>(&'a mut self, k: impl IntoParams<Input = K>, v: impl IntoParams<Input = V>)
    where 
        K: Hash + Eq + 'a + Param,
        V: Param + 'a,
    {
        extern "C" fn insert<'b, K, V>(map: &'b mut HashMap<K, V>, k: K, v: V)
        where K: Hash + Eq + 'b,
            V: 'b,
        {
            map.insert(k, v);
        }

        let f = (insert::<K, V> as extern "C" fn (&'a mut HashMap<K, V>, k: K, v: V)) as usize;

        with_ctx(|ctx| {
            // TODO memoize!
            // TODO: factorize pattern
            let ptr_ty = ctx.module().target_config().pointer_type();
            let mut sig = ctx.module().make_signature();
            sig.params.push(AbiParam::new(ptr_ty));
            K::to_abi_params(&mut sig.params);
            V::to_abi_params(&mut sig.params);
            let sigref = ctx.builder().import_signature(sig);
            let callee = ctx.builder().ins().iconst(ptr_ty, f as i64);
            let mut args = Vec::new();
            (self.get_ref(), k, v).params(ctx, &mut args);
            ctx.builder().ins().call_indirect(sigref, callee, &args);
        });
    }
}
