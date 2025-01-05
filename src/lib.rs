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
mod compare;

pub mod prelude {
    pub use crate::control_flow::if_then_else::{If, FlowControl, ControlFlow, IfCtx};
    pub use crate::compare::Compare;
    pub use crate::val::{Val, AsVal};
    pub use crate::var::Var;

    pub use crate::control_flow::while_loop::{do_while, WhileCtx};
    pub use crate::primitive::ToPrimitive;

    pub use crate::proxy::{Proxy, ProxyMut};

    pub use crate::refs::JitSafe;
    pub use crate::func::IntoHostFn;
    pub use crate::func::Call;
    pub use crate::ctx::Ctx;

    pub use crate::arithmetic::IntMul;

    pub use lego_macros::lego;
}
