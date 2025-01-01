use std::marker::PhantomData;
use std::cell::RefCell;
use std::mem::MaybeUninit;

use cranelift::prelude::{Block, InstBuilder, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};

use crate::primitive::ToPrimitive;
use crate::abi_params::ToAbiParams;
use crate::ctx::Ctx;
use crate::val::Val;
use crate::var::Var;

thread_local! {
    static FN_CTX: RefCell<Option<*mut FnCtx<'static>>> = const { RefCell::new(None) };
}

#[derive(Copy, Clone)]
pub struct Func<P, R> {
    id: FuncId,
    _pth: PhantomData<fn(P) -> R>,
}

pub struct CompiledFunc<P, R> {
    pub(crate) ptr: *const u8,
    pub(crate) _pth: PhantomData<fn(P) -> R>,
}

impl<P, R> Call for CompiledFunc<P, R>
where
    P: Param,
{
    type Input = P;
    type Output = R;

    fn call(&self, input: Self::Input) -> Self::Output {
        let f = unsafe {
            std::mem::transmute::<*const u8, fn(P) -> R>(self.ptr)
        };

        f(input)
    }
}

impl<A, B, R> Call for CompiledFunc<(A, B), R> {
    type Input = (A, B);
    type Output = R;

    fn call(&self, (a, b): Self::Input) -> Self::Output {
        let f = unsafe {
            std::mem::transmute::<*const u8, fn(A, B) -> R>(self.ptr)
        };

        f(a, b)
    }
}

pub trait Call {
    type Input;
    type Output;

    fn call(&self, input: Self::Input) -> Self::Output;
}

pub struct FnCtx<'a> {
    pub(crate) builder: FunctionBuilder<'a>,
    pub(crate) module: &'a mut JITModule,
    pub(crate) var_id: u32,
    pub(crate) current_block: Block,
}

impl<'a> FnCtx<'a> {
    pub(crate) fn declare_var(&mut self) -> Variable {
        self.var_id += 1;
        Variable::from_u32(self.var_id - 1)
    }

    pub(crate) fn create_blocks<const N: usize>(&mut self) -> [Block; N] {
        let mut out = [Block::from_u32(0); N];
        (0..N).for_each(|i| {
            out[i] = self.builder().create_block();
        });

        out
    }

    #[doc(hidden)]
    pub fn builder(&mut self) -> &mut FunctionBuilder<'a> {
        &mut self.builder
    }

    #[doc(hidden)]
    pub fn module(&mut self) -> &mut JITModule {
        self.module
    }
}

pub(crate) fn with_fn_ctx<F, R>(fn_ctx: &mut FnCtx, f: F) -> R
where F: FnOnce() -> R,
{
    FN_CTX.with(|ctx| {
        *ctx.borrow_mut() = Some(fn_ctx as *mut _ as *mut _);
    });
    let ret = f();
    FN_CTX.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
    ret
}

#[doc(hidden)]
pub fn with_ctx<R>(f: impl FnOnce(&mut FnCtx) -> R) -> R {
    FN_CTX.with(|ctx| {
        match *ctx.borrow_mut() {
            Some(ctx) => {
                let ctx = unsafe {
                    &mut *ctx
                };
                f(ctx)
            },
            None => panic!("not in a function build context"),
        }
    })
}

impl<P, R> Func<P, R> {
    pub fn id(&self) -> FuncId {
        self.id
    }
}

impl<P, R> Func<P, R>
where
    P: Params,
    R: Results,
{
    pub(crate) fn new<B>(ctx: &mut Ctx, name: &str, body: B) -> Self
    where B: FnOnce(P::Values) -> R::Results,
    {
        P::to_abi_params(&mut ctx.ctx.func.signature.params);
        R::to_abi_params(&mut ctx.ctx.func.signature.returns);
        let mut builder = FunctionBuilder::new(&mut ctx.ctx.func, &mut ctx.fn_builder_ctx);

        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let mut fn_ctx = FnCtx {
            module: &mut ctx.module,
            builder,
            var_id: 0,
            current_block: block0,
        };

        let params = P::initialize(&mut fn_ctx);

        let ret = with_fn_ctx(&mut fn_ctx, || {
            body(params)
        });

        R::return_(&mut fn_ctx, ret);

        fn_ctx.builder.finalize();

        let func_id = ctx.module.declare_function(name, Linkage::Export, &ctx.ctx.func.signature).unwrap();
        ctx.module.define_function(func_id, &mut ctx.ctx).unwrap();
        ctx.module.clear_context(&mut ctx.ctx);
        ctx.module.finalize_definitions().unwrap();

        Self {
            _pth: PhantomData,
            id: func_id,
        }
    }

    pub fn call<T>(&self, params: T) -> R::Results
        where T: IntoParams<Input = P>
    {
        with_ctx(|ctx| {
            let fn_ref = ctx.module.declare_func_in_func(self.id, ctx.builder.func);
            let mut args = Vec::new();
            params.params(ctx, &mut args);
            let call = ctx.builder.ins().call(fn_ref, &args);
            let results = ctx.builder.inst_results(call);
            R::Results::from_func_ret(results)
        })
    }

    pub fn id_mut(&mut self) -> &mut FuncId {
        &mut self.id
    }
}

pub trait HostFn {
    type Params;
    type Returns: Results;

    fn emit_call(&self, ctx: &mut FnCtx, params: impl IntoParams<Input = Self::Params>) -> <Self::Returns as Results>::Results;
}


pub struct HostFunc<F, T>(F, PhantomData<T>);

impl<F, T> HostFunc<F, T> 
    where Self: HostFn
{
    pub fn call(&self, params: impl IntoParams<Input = <Self as HostFn>::Params>) -> <<Self as HostFn>::Returns as Results>::Results {
        with_ctx(|ctx| {
            self.emit_call(ctx, params)
        })
    }
}

pub trait IntoHostFn<Sig> {
    fn into_host_fn(self) -> HostFunc<Self, Sig> where  Self: Sized;
}

impl<P, R, F> IntoHostFn<(P, R)> for F
where F: FnOnce(P) -> R
{
    fn into_host_fn(self) -> HostFunc<Self, (P, R)> where  Self: Sized {
        HostFunc(self, PhantomData)
    }
}

// adapted from https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=2d064fe8d7f579d0e59df9967861ee7a
trait AssertZeroSized: Sized {
    const ASSERT_ZERO_SIZED: () = [()][size_of::<Self>()];
}

impl<T: Sized> AssertZeroSized for T {}

trait AsFnPtr<P, R> {
    fn as_fn_ptr() -> *const u8;
}

impl<P, R, F: Fn(P) -> R + Copy> AsFnPtr<P, R> for F {
    fn as_fn_ptr() -> *const u8 {
        #[allow(clippy::let_unit_value, path_statements)]
        F::ASSERT_ZERO_SIZED;

        extern "C" fn tramp<F: Fn(P) -> R, P, R>(x: P)  -> R {
            let f: F = unsafe {
                #[allow(clippy::uninit_assumed_init)]
                MaybeUninit::uninit().assume_init()
            };
            f(x)
        }

        (tramp::<F, P, R> as extern "C" fn(P) -> R) as *const u8
    }
}

impl<F, P, R> HostFn for HostFunc<F, (P, R)>
    where
    F: (Fn(P) -> R) + AsFnPtr<P, R>,
    P: Param,
    R: Results,
{
    type Params = P;
    type Returns = R;

    fn emit_call(&self, ctx: &mut FnCtx, params: impl IntoParams<Input = Self::Params>) -> R::Results {
        let ptr_ty = ctx.module().target_config().pointer_type();
        let mut sig = ctx.module().make_signature();
        P::to_abi_params(&mut sig.params);
        R::to_abi_params(&mut sig.returns);
        let sigref = ctx.builder().import_signature(sig);

        let fptr = ctx.builder().ins().iconst(ptr_ty, F::as_fn_ptr() as usize as i64);
        let mut args = Vec::new();
        params.params(ctx, &mut args);
        let call =ctx.builder().ins().call_indirect(sigref, fptr, &args);
        let results = ctx.builder().inst_results(call);
        R::Results::from_func_ret(results)
    }
}

pub trait IntoParams {
    type Input;

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>);
}

impl<T> IntoParams for Val<T> {
    type Input = T;
    fn params(&self, _ctx: &mut FnCtx, out: &mut Vec<Value>) {
        out.push(self.value());
    }
}

impl<A, B> IntoParams for (A, B)
where
    A: IntoParams,
    B: IntoParams,
{
    type Input = (A::Input, B::Input);

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        self.0.params(ctx, out);
        self.1.params(ctx, out);
    }
}

impl<A, B, C> IntoParams for (A, B, C) where 
    A: IntoParams,
    B: IntoParams,
    C: IntoParams,
{
    type Input = (A::Input, B::Input, C::Input);
    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        self.0.params(ctx, out);
        self.1.params(ctx, out);
        self.2.params(ctx, out);
    }
}

pub trait Params: ToAbiParams {
    type Values;

    fn initialize(ctx: &mut FnCtx) -> Self::Values;
}

impl Params for () {
    type Values = ();

    fn initialize(_ctx: &mut FnCtx) -> Self::Values { }
}

impl<T: Param> Params for T {
    type Values = T::Ty;

    fn initialize(ctx: &mut FnCtx) -> Self::Values {
        T::initialize_param_at(ctx, 0)
    }
}

fn initialize_primitive_param_at<T: ToPrimitive>(ctx: &mut FnCtx, idx: usize) -> Var<T> { 
    let variable = ctx.declare_var();
    let val = ctx.builder.block_params(ctx.current_block)[idx];
    ctx.builder.declare_var(variable, T::ty());
    ctx.builder.def_var(variable, val);

    Var::from_variable(variable)
}

pub trait Param: ToAbiParams {
    type Ty;

    fn initialize_param_at(ctx: &mut FnCtx, idx: usize) -> Self::Ty;
}

macro_rules! impl_param_primitives {
    ($($ty:ident $(,)?)*) => {
        $(
            impl Param for $ty {
                type Ty = Var<$ty>;

                fn initialize_param_at(ctx: &mut FnCtx, idx: usize) -> Self::Ty {
                    initialize_primitive_param_at::<$ty>(ctx, idx)
                }
            }
        )*
    };
}

impl_param_primitives![u8, u16, u32, u64, i8, i16, i32, i64];

macro_rules! impl_params_tuples {
    ($($ty:ident $(,)?)*) => {
        impl<$($ty,)*> Params for ($($ty,)*)
        where 
            $($ty: Param,)*
        {
            type Values = ($($ty::Ty,)*);

            #[allow(non_snake_case)] 
            fn initialize(ctx: &mut FnCtx) -> Self::Values {
                let mut idx = 0;
                $(
                    idx += 1;
                    let $ty = $ty::initialize_param_at(ctx, idx - 1);
                )*

                ($($ty,)*) 
            }
        }
    };
}

impl_params_tuples!(A, B);
impl_params_tuples!(A, B, C);

pub trait FromFuncRet {
    fn from_func_ret(vals: &[Value]) -> Self;
}

impl<T> FromFuncRet for Val<T> {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert_eq!(vals.len(), 1);
        Val::from_value(vals[0]) 
    }
}

impl FromFuncRet for () {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert!(vals.is_empty());
    }
}

pub trait Results: ToAbiParams {
    type Results: FromFuncRet;

    fn return_(ctx: &mut FnCtx, results: Self::Results);
}

impl Results for u32 {
    type Results = Val<u32>;

    fn return_(ctx: &mut FnCtx, results: Self::Results) {
        ctx.builder.ins().return_(&[results.value()]);
    }
}

impl Results for () {
    type Results = ();

    fn return_(ctx: &mut FnCtx, _results: Self::Results) {
        ctx.builder.ins().return_(&[]);
    }
}

