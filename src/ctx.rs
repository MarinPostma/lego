use std::marker::PhantomData;
use std::collections::HashMap;

use cranelift::prelude::Configurable as _;
use cranelift_codegen::settings;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Module as _;

use crate::{func::{CompiledFunc, Func, Params, Results}, prelude::ControlFlow};

pub struct Ctx {
    pub(crate) fn_builder_ctx: FunctionBuilderContext,
    pub(crate) module: JITModule,
    pub(crate) ctx: cranelift::prelude::codegen::Context,
}

#[derive(Default)]
pub struct CtxBuilder {
    registered_functions: Vec<NamedHostFn>,
}

#[doc(hidden)]
pub struct NamedHostFn {
    pub name: &'static str,
    pub ptr: *const u8,
}

#[macro_export]
macro_rules! host_fns {
    ($($name:expr => $as:ty $(,)?)*) => {
        {
            use $crate::func::HostFn;
            [
                $($crate::ctx::NamedHostFn {
                    name: stringify!($name),
                    ptr: ($name as $as).to_fn_ptr(),
                }),*
            ]
        }
    };
}


impl CtxBuilder {
    pub fn register_host_functions(&mut self, f: impl IntoIterator<Item = NamedHostFn>) {
        self.registered_functions.extend(f);
    }

    pub fn build(self) -> Ctx {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        // flag_builder.set("opt_level", "none").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        let mut host_fn_map = HashMap::with_capacity(self.registered_functions.len());
        for NamedHostFn { name, ptr } in self.registered_functions {
            builder.symbol(name, ptr);
            host_fn_map.insert(ptr as usize, name);
        }

        let module = JITModule::new(builder);

        Ctx {
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        }

    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

impl Ctx {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn func<P, R>(&mut self, body: impl FnOnce(P::Values) -> ControlFlow<(), R::Results>) -> Func<P, R>
    where
        P: Params,
        R: Results,
    {
        Func::new(self, body)
    }

    pub fn get_compiled_function<P, R>(&self, f: Func<P, R>) -> CompiledFunc<P, R> {
        let ptr = self.module.get_finalized_function(f.id());
        CompiledFunc { ptr, _pth: PhantomData }
    }

    pub fn builder() -> CtxBuilder {
        CtxBuilder::default()
    }

    pub fn ctx(&self) -> &cranelift::prelude::codegen::Context {
        &self.ctx
    }
}

