use cranelift::prelude::{Block, InstBuilder};

use crate::{
    func::{with_ctx, FnCtx},
    val::Val,
};

use super::BlockRet;

impl Val<bool> {
    pub fn then<T, E, B>(self, f: T) -> B
    where
        T: FnOnce() -> (B, E),
        E: FnOnce() -> B,
        B: BlockRet,
    {
        let mut params = Vec::new();
        let [then_block, else_block, merge_block] = with_ctx(make_cond_blocks::<B>);

        with_ctx(|ctx| {
            let b = ctx.builder();
            b.ins().brif(self.value(), then_block, &[], else_block, &[]);
            b.switch_to_block(then_block);
            b.seal_block(then_block);
        });

        let (then_val, else_fn) = f();

        with_ctx(|ctx| {
            then_val.to_block_values(&mut params);
            ctx.builder()
                .ins()
                .jump(merge_block, &params);
            ctx.builder().switch_to_block(else_block);
            ctx.builder().seal_block(else_block);
            params.clear();
        });

        let else_val = else_fn();

        with_ctx(|ctx| {
            else_val.to_block_values(&mut params);
            ctx.builder()
                .ins()
                .jump(merge_block, &params);
            params.clear();

            let b = ctx.builder();
            b.switch_to_block(merge_block);
            b.seal_block(merge_block);
            B::read_from_ret(&mut ctx.builder().block_params(merge_block).iter().copied())
        })
    }
}

fn make_cond_blocks<T: BlockRet>(ctx: &mut FnCtx) -> [Block; 3] {
    let [then_block, else_block, merge_block] = ctx.create_blocks();
    T::push_param_ty(ctx, merge_block);
    [then_block, else_block, merge_block]
}
