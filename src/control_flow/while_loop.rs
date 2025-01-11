use cranelift::prelude::InstBuilder;

use crate::{func::with_ctx, val::Val};

use super::BlockRet;

pub fn do_while2<C, B, BR>(param: BR, f: C) -> BR
where
    C: FnOnce() -> (Val<bool>, B),
    B: FnOnce(BR) -> BR,
    BR: BlockRet,
{
    let [header_block, body_block, exit_block] = with_ctx(|ctx| {
        let [header_block, body_block, exit_block] = ctx.create_blocks();

        BR::push_param_ty(ctx, exit_block);
        BR::push_param_ty(ctx, body_block);
        BR::push_param_ty(ctx, header_block);
        let builder = ctx.builder();
        builder.ins().jump(header_block, &param.to_block_values());
        builder.switch_to_block(header_block);
        [header_block, body_block, exit_block]
    });

    let (header_val, body_fn) = f();

    let param = with_ctx(|ctx| {
        let ret = BR::read_from_ret(ctx, header_block);
        let builder = ctx.builder();
        let params = ret.to_block_values();
        builder
            .ins()
            .brif(header_val.value(), body_block, &params, exit_block, &params);
        builder.switch_to_block(body_block);
        builder.seal_block(body_block);
        BR::read_from_ret(ctx, body_block)
    });

    let body_val = (body_fn)(param);

    with_ctx(|ctx| {
        // BR::jump_to(body_val, ctx, header_block);
        let builder = ctx.builder();
        builder
            .ins()
            .jump(header_block, &body_val.to_block_values());
        builder.switch_to_block(exit_block);
        builder.seal_block(header_block);
        builder.seal_block(exit_block);
        BR::read_from_ret(ctx, exit_block)
    })
}
