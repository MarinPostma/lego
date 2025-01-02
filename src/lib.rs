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
    pub use crate::control_flow::while_loop::{While, Body};
    pub use crate::control_flow::if_then_else::{If, Then, Else};
    pub use crate::compare::Compare;
    pub use crate::val::Val;
    pub use crate::var::Var;

    pub use crate::proxy::{Proxy, ProxyMut};

    pub use crate::refs::JitSafe;
    pub use crate::func::IntoHostFn;
    pub use crate::func::Call;
    pub use crate::ctx::Ctx;

    pub use lego_macros::{lego_if, lego_while};
}
