use cranelift::prelude::{Block, Value};

use crate::func::FnCtx;
use crate::primitive::Primitive;
use crate::proxy::Ref;
use crate::val::Val;

mod then;
pub mod while_loop;

pub trait BlockRet {
    /// push param ty for the passed block
    fn push_param_ty(ctx: &mut FnCtx, block: Block);
    fn read_from_ret(block_params: &mut impl Iterator<Item = Value>) -> Self;
    fn to_block_values(&self, out: &mut Vec<Value>);
}

impl BlockRet for () {
    fn push_param_ty(_ctx: &mut FnCtx, _block: Block) {}

    fn read_from_ret(_block_params: &mut impl Iterator<Item = Value>) -> Self {}

    fn to_block_values(&self, _out: &mut Vec<Value>) { }
}

impl<T: Primitive> BlockRet for Val<T> {
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        ctx.builder().append_block_param(block, T::ty());
    }

    fn read_from_ret(block_params: &mut impl Iterator<Item = Value>) -> Self {
        Val::from_value(block_params.next().unwrap())
    }

    fn to_block_values(&self, out: &mut Vec<Value>) {
        out.push(self.value());
    }
}

impl<T> BlockRet for Ref<'_, T> {
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        ctx.builder().append_block_param(block, <*const T>::ty());
    }

    fn read_from_ret(block_params: &mut impl Iterator<Item = Value>) -> Self {
        let addr = Val::from_value(block_params.next().unwrap());
        Ref::new(addr)
    }

    fn to_block_values(&self, out: &mut Vec<Value>) {
        out.push(self.addr.value());
    }
}

impl<T, U> BlockRet for (T, U)
    where
    T: BlockRet, U: BlockRet,
{
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        T::push_param_ty(ctx, block);
        U::push_param_ty(ctx, block);
    }

    fn read_from_ret(block_params: &mut impl Iterator<Item = Value>) -> Self {
        (
            T::read_from_ret(block_params),
            U::read_from_ret(block_params),
        )
    }

    fn to_block_values(&self, out: &mut Vec<Value>) {
        self.0.to_block_values(out);
        self.1.to_block_values(out);
    }
}

impl<T, U, V> BlockRet for (T, U, V)
    where
    T: BlockRet, U: BlockRet, V: BlockRet,
{
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        T::push_param_ty(ctx, block);
        U::push_param_ty(ctx, block);
        V::push_param_ty(ctx, block);
    }

    fn read_from_ret(block_params: &mut impl Iterator<Item = Value>) -> Self {
        (
            T::read_from_ret(block_params),
            U::read_from_ret(block_params),
            V::read_from_ret(block_params),
        )
    }

    fn to_block_values(&self, out: &mut Vec<Value>) {
        self.0.to_block_values(out);
        self.1.to_block_values(out);
        self.2.to_block_values(out);
    }
}
