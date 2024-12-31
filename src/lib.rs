pub mod ctx;
pub mod types;
pub mod func;
pub mod arithmetic;
pub mod datatype;
pub mod refs;
pub mod proxy;
pub mod control_flow;
pub mod var;

pub mod prelude {
    pub use crate::control_flow::while_loop::{While, Body};
    pub use crate::control_flow::if_then_else::{If, Then, Else};

    pub use crate::proxy::{Proxy, ProxyMut};

    pub use crate::refs::JitSafe;
}
