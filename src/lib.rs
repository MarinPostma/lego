mod abi_params;
mod arithmetic;
mod cmp;
mod control_flow;
mod ctx;
pub mod ffi;
mod func;
mod iterator;
mod macros;
mod primitive;
mod proxy;
mod refs;
mod slice;
mod val;
mod var;
mod vec;

pub mod prelude {
    // pub use crate::control_flow::if_then_else::{If, FlowControl, ControlFlow, IfCtx};
    pub use crate::cmp::Compare;
    pub use crate::val::{AsVal, Val};
    pub use crate::var::Var;

    // pub use crate::control_flow::while_loop::{do_while, WhileCtx};
    pub use crate::abi_params::ToAbiParams;
    pub use crate::primitive::Primitive;

    pub use crate::proxy::{Proxy, Ref, RefMut};
    pub use crate::slice::Slice;

    pub use crate::ctx::Ctx;
    pub use crate::func::Call;
    pub use crate::func::IntoHostFn;
    pub use crate::func::Param;
    pub use crate::refs::JitSafe;

    pub use crate::arithmetic::*;
    pub use crate::iterator::{IntoJiter, JIterator};
    pub use crate::func::CompiledFunc;

    pub use lego_macros::lego;
}
