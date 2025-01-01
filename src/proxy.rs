// use std::collections::HashMap;
// use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::{InstBuilder as _, MemFlags};
use cranelift::prelude::*;
// use cranelift_module::Module;

use crate::func::{with_ctx, FnCtx};
use crate::primitive::ToPrimitive;
use crate::val::{AsVal, Val};

impl<T: ToPrimitive> Proxy<T> {
    fn load(&self, ctx: &mut FnCtx) -> Val<T> {
        Val::from_value(ctx.builder.ins().load(T::ty(), MemFlags::new(), self.addr, self.offset))
    }

    pub fn get(&self) -> Val<T> {
        with_ctx(|ctx| {
            self.load(ctx)
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

impl<T> ProxyMut<T> {
    fn store(&mut self, ctx: &mut FnCtx, val: Value) {
        ctx.builder.ins().store(MemFlags::new(), val, self.addr, self.offset);
    }

    pub fn put(&mut self, val: impl AsVal<Ty = T>) {
        with_ctx(|ctx| {
            let val = val.as_val(ctx);
            self.store(ctx, val.value());
        });
    }
}

#[derive(Clone, Copy)]
pub struct Proxy<T> {
    addr: Value,
    offset: i32,
    _pth: PhantomData<T>,
}

impl<T: ToPrimitive> AsVal for Proxy<T> {
    type Ty = T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        self.load(ctx)
    }
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

impl<'a, T> AsVal for &'a Proxy<T> {
    type Ty = &'a T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        let addr = if self.offset() != 0 {
            ctx.builder().ins().iadd_imm(self.addr(), self.offset() as i64)
        } else {
            self.addr()
        };
        Val::from_value(addr)
    }
}

impl<'a, T> AsVal for &'a mut ProxyMut<T> {
    type Ty = &'a mut T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        let p = &self.0;
        Val::from_value(p.as_val(ctx).value())
    }
}

// impl<K, V> ProxyMut<HashMap<K, V>>
// {
//     pub fn insert<'a>(&'a mut self, k: impl IntoParams<Input = K>, v: impl IntoParams<Input = V>)
//     where 
//         K: Hash + Eq + 'a + Param,
//         V: Param + 'a,
//     {
//         extern "C" fn insert<'b, K, V>(map: &'b mut HashMap<K, V>, k: K, v: V)
//         where K: Hash + Eq + 'b,
//             V: 'b,
//         {
//             map.insert(k, v);
//         }
//
//         let f = (insert::<K, V> as extern "C" fn (&'a mut HashMap<K, V>, k: K, v: V)) as usize;
//
//         with_ctx(|ctx| {
//             // TODO memoize!
//             // TODO: factorize pattern
//             let ptr_ty = ctx.module().target_config().pointer_type();
//             let mut sig = ctx.module().make_signature();
//             sig.params.push(AbiParam::new(ptr_ty));
//             K::to_abi_params(&mut sig.params);
//             V::to_abi_params(&mut sig.params);
//             let sigref = ctx.builder().import_signature(sig);
//             let callee = ctx.builder().ins().iconst(ptr_ty, f as i64);
//             let mut args = Vec::new();
//             // FIXME: There is a hole here: self.get_ref() works!
//             (self.get_mut(), k, v).params(ctx, &mut args);
//             ctx.builder().ins().call_indirect(sigref, callee, &args);
//         });
//     }
// }
