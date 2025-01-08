use cranelift::prelude::AbiParam;
use cranelift::prelude::types::I64;
use cranelift_module::Module;

use crate::func::Param;
use crate::proxy::{Ref, RefMut};
use crate::abi_params::ToAbiParams;
use crate::val::Val;

/// Implented for types that can be shared with JIT
/// # Safety
/// this should not be implemented manually, use proxy! isntead
pub unsafe trait JitSafe {}

impl<T> ToAbiParams for &T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        // fixme: we actually need to pass the real pointer size
        params.push(AbiParam::new(I64));
    }
}

impl<T> ToAbiParams for &mut T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        // fixme: we actually need to pass the real pointer size
        params.push(AbiParam::new(I64));
    }
}

impl<T> Param for &T {
    type Ty = Ref<T>;

    fn initialize_param_at(ctx: &mut crate::func::FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        let variable = ctx.declare_var();
        let val = ctx.builder.block_params(ctx.current_block)[idxs.next().unwrap()];
        ctx.builder.declare_var(variable, ctx.module.target_config().pointer_type());
        ctx.builder.def_var(variable, val);

        Ref::new(Val::from_value(val), 0)
    }
}

impl<T> Param for &mut T {
    type Ty = RefMut<T>;

    fn initialize_param_at(ctx: &mut crate::func::FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        let variable = ctx.declare_var();
        let val = ctx.builder.block_params(ctx.current_block)[idxs.next().unwrap()];
        ctx.builder.declare_var(variable, ctx.module.target_config().pointer_type());
        ctx.builder.def_var(variable, val);

        RefMut::new(Val::from_value(val), 0)
    }
}
