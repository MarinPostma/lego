use cranelift::prelude::{Block, InstBuilder as _};

use crate::primitive::Primitive;
use crate::val::Val;
use crate::func::FnCtx;

pub mod if_then_else;
pub mod while_loop;

pub trait BlockRet {
    /// push param ty for the passed block
    fn push_param_ty(ctx: &mut FnCtx, block: Block);
    fn jump_to(self, ctx: &mut FnCtx, block: Block);
    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self;
}

impl BlockRet for () {
    fn push_param_ty(_ctx: &mut FnCtx, _block: Block) { }

    fn jump_to(self, ctx: &mut FnCtx, block: Block) { 
        ctx.builder().ins().jump(block, &[]);
    }

    fn read_from_ret(_ctx: &mut FnCtx, _block: Block) -> Self { }
}

impl<T: Primitive> BlockRet for Val<T> {
    fn push_param_ty(ctx: &mut FnCtx, block: Block) {
        ctx.builder().append_block_param(block, T::ty());
    }

    fn jump_to(self, ctx: &mut FnCtx, block: Block) {
        ctx.builder().ins().jump(block, &[self.value()]);
    }

    fn read_from_ret(ctx: &mut FnCtx, block: Block) -> Self {
        Val::from_value(ctx.builder().block_params(block)[0])
    }
}
