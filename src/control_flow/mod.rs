use cranelift::prelude::{Block, Value};

use crate::func::FnCtx;
use crate::primitive::Primitive;
use crate::val::Val;

mod then;
pub mod while_loop;

pub trait BlockRet {
    /// push param ty for the passed block
    fn push_param_ty(ctx: &mut FnCtx, block: Block);
    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self;
    fn to_block_values(&self) -> Vec<Value>;
}

impl BlockRet for () {
    fn push_param_ty(_ctx: &mut FnCtx, _block: Block) {}

    fn read_from_ret(_ctx: &mut FnCtx, _block: Block) -> Self {}

    fn to_block_values(&self) -> Vec<Value> {
        Vec::new()
    }
}

impl<T: Primitive> BlockRet for Val<T> {
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        ctx.builder().append_block_param(block, T::ty());
    }

    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self {
        Val::from_value(ctx.builder().block_params(block)[0])
    }

    fn to_block_values(&self) -> Vec<Value> {
        vec![self.value()]
    }
}
