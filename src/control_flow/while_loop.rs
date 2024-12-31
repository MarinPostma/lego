use cranelift::prelude::{Block, InstBuilder};

use crate::func::with_ctx;

use super::{if_then_else::Branch, Cond};

pub struct While {
    header_block: Block,
    exit_block: Block,
    has_body: bool,
}

pub struct Body<B>(pub B);

impl<B> Branch for Body<B>
where
    B: FnOnce(),
{
    type Output = ();

    fn eval(self) -> Self::Output {
        (self.0)()
    }
}


impl While {
    #[must_use]
    #[doc(hidden)]
    pub fn new<C>(header: C) -> Self
    where C: Cond,
    {
        let [header_block, body_block, exit_block] = with_ctx(|ctx| {
            let [header_block, body_block, exit_block] = ctx.create_blocks();

            let builder = ctx.builder();
            builder.ins().jump(header_block, &[]);
            builder.switch_to_block(header_block);
            [header_block, body_block, exit_block]
        });

        let header_val = header.eval();

        with_ctx(|ctx| {
            let builder = ctx.builder();
            builder.ins().brif(header_val.value(), body_block, &[], exit_block, &[]);
            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
        });

        Self {
            header_block,
            exit_block,
            has_body: false,
        }
    }

    pub fn body<B>(mut self, body: Body<B>)
    where Body<B>: Branch<Output = ()>
    {
        body.eval();

        with_ctx(|ctx| {
            let builder = ctx.builder();
            builder.ins().jump(self.header_block, &[]);
            builder.switch_to_block(self.exit_block);
            builder.seal_block(self.header_block);
            builder.seal_block(self.exit_block);
        });

        self.has_body = true;
    }
}

impl Drop for While {
    fn drop(&mut self) {
        assert!(self.has_body, "loop created without a body is invalid")
    }
}


#[macro_export]
macro_rules! lego_while {
    (while ($header:expr) { $($body:tt)* }) => {
        {
            $crate::prelude::While::new(|| { $header })
                .body($crate::prelude::Body(|| { $($body)* }))
        }
    };
}
