mod macros;
mod primitive;
mod ctx;
mod func;
mod arithmetic;
mod datatype;
mod refs;
mod proxy;
mod control_flow;
mod var;
mod val;
mod abi_params;
mod cmp;
pub mod ffi;
mod vec;

pub mod prelude {
    pub use crate::control_flow::if_then_else::{If, FlowControl, ControlFlow, IfCtx};
    pub use crate::cmp::Compare;
    pub use crate::val::{Val, AsVal};
    pub use crate::var::Var;

    pub use crate::control_flow::while_loop::{do_while, WhileCtx};
    pub use crate::primitive::ToPrimitive;
    pub use crate::abi_params::ToAbiParams;

    pub use crate::proxy::{Ref, RefMut, Proxy};
    pub use crate::proxy::Slice;

    pub use crate::refs::JitSafe;
    pub use crate::func::IntoHostFn;
    pub use crate::func::Call;
    pub use crate::func::Param;
    pub use crate::ctx::Ctx;

    pub use crate::arithmetic::*;

    pub use lego_macros::lego;
}
