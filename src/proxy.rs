use std::marker::PhantomData;
use std::ops::Deref;

use cranelift::prelude::*;
use cranelift_module::Module;

use crate::func::{with_ctx, FnCtx, IntoHostFn};
use crate::primitive::Primitive;
use crate::val::{AsVal, Val};

impl<T: Primitive> Ref<'_, T> {
    fn load(&self, ctx: &mut FnCtx) -> Val<T> {
        Val::from_value(ctx.builder.ins().load(
            T::ty(),
            MemFlags::new(),
            self.addr.value(),
            0,
        ))
    }

    pub fn get(&self) -> Val<T> {
        with_ctx(|ctx| self.load(ctx))
    }
}

#[repr(transparent)]
pub struct RefMut<'a, T>(Ref<'a, T>);

impl<T> RefMut<'_, T> {
    #[doc(hidden)]
    pub fn new(addr: Val<*mut T>) -> Self {
        Self(Ref::new(addr.into()))
    }
}

impl<'a, T> Deref for RefMut<'a, T> {
    type Target = Ref<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> RefMut<'_, T> {
    fn store(&mut self, ctx: &mut FnCtx, val: Value) {
        ctx.builder
            .ins()
            .store(MemFlags::new(), val, self.addr.value(), 0);
    }

    pub fn put(&mut self, val: impl AsVal<Ty = T>) {
        with_ctx(|ctx| {
            let val = val.as_val(ctx);
            self.store(ctx, val.value());
        });
    }
}

#[derive(Clone, Copy)]
pub struct Ref<'a, T> {
    pub(crate) addr: Val<*const T>,
    _pth: PhantomData<&'a T>,
}

impl<'a, T> AsVal for Ref<'a, T> {
    type Ty = &'a T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        // safety: we trivially know that this is a &T
        unsafe { self.addr(ctx).transmute() }
    }
}

impl<'a, T> AsVal for RefMut<'a, T> {
    type Ty = &'a mut T;

    fn as_val(&self, ctx: &mut FnCtx) -> Val<Self::Ty> {
        // safety: we trivially know that this is a &mut T
        unsafe { self.addr(ctx).transmute() }
    }
}

impl<T> Ref<'_, T> {
    pub fn deref(&self) -> Val<T>
    where
        T: Primitive,
    {
        with_ctx(|ctx| self.load(ctx))
    }

    #[doc(hidden)]
    pub fn base(&self) -> Val<usize> {
        self.addr.into()
    }


    fn addr(&self, _ctx: &mut FnCtx) -> Val<usize> {
        self.addr.into()
    }

    #[doc(hidden)]
    pub fn new(addr: Val<*const T>) -> Self {
        Self {
            addr,
            _pth: PhantomData,
        }
    }
}

pub struct Ptr<T> {
    pub(crate) addr: Val<*const T>,
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Ptr<T> {}
impl<T> Ptr<T> {
    pub(crate) fn from_value(addr: Val<*const T>) -> Self {
        Self { addr }
    }
}

pub struct PtrMut<T> {
    pub(crate) addr: Val<*mut T>,
}

impl<T> Clone for PtrMut<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for PtrMut<T> {}

impl<T> PtrMut<T> {
    pub(crate) fn from_value(addr: Val<*mut T>) -> Self {
        Self { addr }
    }
}

pub struct Proxy<T> {
    pub ptr: PtrMut<T>,
}

fn drop_ptr<T>(p: *mut T) {
    unsafe {
        std::ptr::read(p);
    }
}

impl<T> Proxy<T> {
    pub fn get_mut(&mut self) -> RefMut<T> {
        RefMut::new(self.ptr.addr)
    }

    pub fn ctor(ctor: fn() -> T) -> Self {
        let tramp = (|slot: *mut T, ctor: *mut u8| unsafe {
            let ctor = std::mem::transmute::<*mut u8, fn() -> T>(ctor);
            slot.write(ctor());
        })
        .into_host_fn();

        let ctor = ctor as *mut u8;
        let val = Val::new(ctor);

        let addr = with_ctx(|ctx| {
            let data = StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size_of::<T>() as u32,
                align_of::<T>().ilog2() as u8,
            );
            let slot = ctx.builder().create_sized_stack_slot(data);
            let ptr = ctx.module().target_config().pointer_type();
            let addr = ctx.builder().ins().stack_addr(ptr, slot, 0);
            addr
        });

        let mut ptr = PtrMut {
            addr: Val::from_value(addr),
        };
        tramp.call((&mut ptr, val));

        Self { ptr }
    }

    pub fn get_ref(&self) -> Ref<T> {
        Ref::new(self.ptr.addr.into())
    }
}

impl<T> Drop for Proxy<T> {
    fn drop(&mut self) {
        let f = drop_ptr::<T>.into_host_fn();
        f.call(&mut self.ptr)
    }
}
