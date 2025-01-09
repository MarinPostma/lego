use cranelift::prelude::{InstBuilder, IntCC};

use crate::primitive::Primitive;
use crate::val::{AsVal, Val};
use crate::func::with_ctx;

impl<T, U, P> Compare<&U> for &T 
where
    T: AsVal<Ty = P>,
    U: AsVal<Ty = P>,
    P: Primitive,
{
    fn eq(self, other: &U) -> Val<bool> {
        with_ctx(|ctx| {
            let lhs = self.as_val(ctx);
            let rhs = other.as_val(ctx);
            let val = ctx.builder().ins().icmp(IntCC::Equal, lhs.value(), rhs.value());
            Val::from_value(val)
        })
    }

    fn neq(self, other: &U) -> Val<bool> {
        with_ctx(|ctx| {
            let lhs = self.as_val(ctx);
            let rhs = other.as_val(ctx);
            let val = ctx.builder().ins().icmp(IntCC::NotEqual, lhs.value(), rhs.value());
            Val::from_value(val)
        })
    }
}

pub trait Compare<Rhs = Self> {
    fn eq(self, other: Rhs) -> Val<bool>;
    fn neq(self, other: Rhs) -> Val<bool>;
}
