use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use cranelift::prelude::{Block, InstBuilder, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Module};

// use crate::prelude::ControlFlow;
use crate::abi_params::ToAbiParams;
use crate::ctx::Ctx;
use crate::primitive::Primitive;
use crate::proxy::{Ptr, PtrMut};
use crate::val::{AsVal, Val};
use crate::var::Var;
use crate::{for_all_primitives, for_all_tuples, maybe_paren};

thread_local! {
    static FN_CTX: RefCell<Option<*mut FnCtx<'static>>> = const { RefCell::new(None) };
}

#[derive(Copy, Clone)]
pub struct Func<P, R> {
    id: FuncId,
    _pth: PhantomData<fn(P) -> R>,
}

pub struct CompiledFunc<'a, P, R> {
    pub(crate) ptr: *const u8,
    pub(crate) _pth: PhantomData<&'a fn(P) -> R>,
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
where
    F: FnOnce() -> R,
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
    FN_CTX.with(|ctx| match *ctx.borrow_mut() {
        Some(ctx) => {
            let ctx = unsafe { &mut *ctx };
            f(ctx)
        }
        None => panic!("not in a function build context"),
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
    pub(crate) fn new<B>(ctx: &mut Ctx, body: B) -> Self
    where
        B: FnOnce(P::Values) -> R::Results,
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

        let ret = with_fn_ctx(&mut fn_ctx, || body(params));

        ret.return_(&mut fn_ctx);

        fn_ctx.builder.finalize();

        let func_id = ctx
            .module
            .declare_anonymous_function(&ctx.ctx.func.signature)
            .unwrap();

        ctx.module.define_function(func_id, &mut ctx.ctx).unwrap();
        println!(
            "{}",
            ctx.ctx
                .compiled_code()
                .as_ref()
                .unwrap()
                .vcode
                .as_ref()
                .unwrap()
        );
        ctx.module.clear_context(&mut ctx.ctx);
        ctx.module.finalize_definitions().unwrap();

        Self {
            _pth: PhantomData,
            id: func_id,
        }
    }

    pub fn call<T>(&self, params: T) -> R::Results
    where
        T: IntoParams<Input = P>,
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

    fn emit_call(
        &self,
        ctx: &mut FnCtx,
        params: impl IntoParams<Input = Self::Params>,
    ) -> <Self::Returns as Results>::Results;
}

#[derive(Debug, Copy, Clone)]
pub struct HostFunc<F, P, R>(F, PhantomData<fn(P) -> R>);

impl<F, P, R> HostFunc<F, P, R>
where
    Self: HostFn,
{
    pub fn call(
        &self,
        params: impl IntoParams<Input = <Self as HostFn>::Params>,
    ) -> <<Self as HostFn>::Returns as Results>::Results {
        with_ctx(|ctx| self.emit_call(ctx, params))
    }
}

pub trait IntoHostFn<P, R> {
    fn into_host_fn(self) -> HostFunc<Self, P, R>
    where
        Self: Sized;
}

impl<A, R, F> IntoHostFn<(A,), R> for F
where
    F: FnOnce(A) -> R,
{
    fn into_host_fn(self) -> HostFunc<Self, (A,), R>
    where
        Self: Sized,
    {
        HostFunc(self, PhantomData)
    }
}

impl<A, B, R, F> IntoHostFn<(A, B), R> for F
where
    F: FnOnce(A, B) -> R,
{
    fn into_host_fn(self) -> HostFunc<Self, (A, B), R>
    where
        Self: Sized,
    {
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

macro_rules! impl_as_fn_ptr {
    ($($ty:ident $(,)?)*) => {
        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        impl<RET, FUN: Fn($($ty,)*) -> RET + Copy, $($ty),* > AsFnPtr<($($ty),*), RET> for FUN
        where
            $($ty: Param),*
        {
            fn as_fn_ptr() -> *const u8 {
                #[allow(clippy::let_unit_value, path_statements)]
                FUN::ASSERT_ZERO_SIZED;

                extern "C" fn tramp<FUN: Fn($($ty),*) -> RET, RET, $($ty,)*>($($ty: $ty),*)  -> RET {
                    // F is zero-size, we can create it out of thin air
                    let f: FUN = unsafe {
                        #[allow(clippy::uninit_assumed_init)]
                        MaybeUninit::uninit().assume_init()
                    };
                    f($($ty,)*)
                }

                (tramp::<FUN , RET, $($ty),*> as extern "C" fn($($ty),*) -> RET) as *const u8
            }
        }

    };
}

#[allow(unused_parens)]
#[allow(non_snake_case)]
impl<RET, FUN: Fn(A) -> RET + Copy, A> AsFnPtr<(A,), RET> for FUN
where
    A: Param,
{
    fn as_fn_ptr() -> *const u8 {
        #[allow(clippy::let_unit_value, path_statements)]
        FUN::ASSERT_ZERO_SIZED;
        extern "C" fn tramp<FUN: Fn(A) -> RET, RET, A>(A: A) -> RET {
            let f: FUN = unsafe {
                #[allow(clippy::uninit_assumed_init)]
                MaybeUninit::uninit().assume_init()
            };
            f(A)
        }
        (tramp::<FUN, RET, A> as extern "C" fn(A) -> RET) as *const u8
    }
}
impl_as_fn_ptr!(A, B);
impl_as_fn_ptr!(A, B, C);
impl_as_fn_ptr!(A, B, C, D);
impl_as_fn_ptr!(A, B, C, D, E);
impl_as_fn_ptr!(A, B, C, D, E, F);
impl_as_fn_ptr!(A, B, C, D, E, F, G);

impl<F, A, R> HostFn for HostFunc<F, (A,), R>
where
    F: (Fn(A) -> R) + AsFnPtr<(A,), R>,
    A: Param,
    R: Results,
{
    type Params = A;
    type Returns = R;

    fn emit_call(
        &self,
        ctx: &mut FnCtx,
        params: impl IntoParams<Input = Self::Params>,
    ) -> R::Results {
        let ptr_ty = ctx.module().target_config().pointer_type();
        let mut sig = ctx.module().make_signature();
        A::to_abi_params(&mut sig.params);
        R::to_abi_params(&mut sig.returns);
        let sigref = ctx.builder().import_signature(sig);

        let fptr = ctx
            .builder()
            .ins()
            .iconst(ptr_ty, F::as_fn_ptr() as usize as i64);
        let mut args = Vec::new();
        params.params(ctx, &mut args);
        let call = ctx.builder().ins().call_indirect(sigref, fptr, &args);
        let results = ctx.builder().inst_results(call);
        R::Results::from_func_ret(results)
    }
}

impl<F, A, B, R> HostFn for HostFunc<F, (A, B), R>
where
    F: (Fn(A, B) -> R) + AsFnPtr<(A, B), R>,
    A: Param,
    B: Param,
    R: Results,
{
    type Params = (A, B);
    type Returns = R;

    fn emit_call(
        &self,
        ctx: &mut FnCtx,
        params: impl IntoParams<Input = Self::Params>,
    ) -> R::Results {
        let ptr_ty = ctx.module().target_config().pointer_type();
        let mut sig = ctx.module().make_signature();
        A::to_abi_params(&mut sig.params);
        B::to_abi_params(&mut sig.params);
        R::to_abi_params(&mut sig.returns);
        let sigref = ctx.builder().import_signature(sig);

        let fptr = ctx
            .builder()
            .ins()
            .iconst(ptr_ty, F::as_fn_ptr() as usize as i64);
        let mut args = Vec::new();
        params.params(ctx, &mut args);
        let call = ctx.builder().ins().call_indirect(sigref, fptr, &args);
        let results = ctx.builder().inst_results(call);
        R::Results::from_func_ret(results)
    }
}

pub trait IntoParams {
    type Input;

    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>);
}

macro_rules! impl_into_params_for_as_val {
    ($($ty:ident $(,)?)*) => {
        #[allow(unused_parens, non_snake_case)]
        impl<$($ty),*> IntoParams for maybe_paren!($($ty),*)
        where
            $($ty: AsVal),*
        {
            type Input = maybe_paren!($($ty::Ty),*);

            fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
                let ($($ty),*) = self;
                $(
                    out.push($ty.as_val(ctx).value());
                )*
            }
        }
    };
}

for_all_tuples!(impl_into_params_for_as_val);

pub trait Params: ToAbiParams {
    type Values;

    fn initialize(ctx: &mut FnCtx) -> Self::Values;
}

impl Params for () {
    type Values = ();

    fn initialize(_ctx: &mut FnCtx) -> Self::Values {}
}

fn initialize_primitive_param_at<T: Primitive>(ctx: &mut FnCtx, idx: usize) -> Var<T> {
    let variable = ctx.declare_var();
    let val = ctx.builder.block_params(ctx.current_block)[idx];
    ctx.builder.declare_var(variable, T::ty());
    ctx.builder.def_var(variable, val);

    Var::from_variable(variable)
}

pub trait Param: ToAbiParams {
    type Ty;

    fn initialize_param_at(ctx: &mut FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty;
}

macro_rules! impl_param_primitive {
    ($ty:ident) => {
        impl Param for $ty {
            type Ty = Var<$ty>;

            fn initialize_param_at(
                ctx: &mut FnCtx,
                idxs: &mut impl Iterator<Item = usize>,
            ) -> Self::Ty {
                initialize_primitive_param_at::<$ty>(ctx, idxs.next().unwrap())
            }
        }
    };
}

for_all_primitives!(impl_param_primitive);

impl<T> Param for *mut T {
    type Ty = PtrMut<T>;

    fn initialize_param_at(ctx: &mut FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        let val = ctx.builder.block_params(ctx.current_block)[idxs.next().unwrap()];
        let val = Val::from_value(val);
        PtrMut::from_value(val)
    }
}

impl<T> Param for *const T {
    type Ty = Ptr<T>;

    fn initialize_param_at(ctx: &mut FnCtx, idxs: &mut impl Iterator<Item = usize>) -> Self::Ty {
        let val = ctx.builder.block_params(ctx.current_block)[idxs.next().unwrap()];
        let val = Val::from_value(val);
        Ptr::from_value(val)
    }
}

macro_rules! impl_params_tuples {
    ($($ty:ident $(,)?)*) => {
        impl<$($ty,)*> Params for maybe_paren!($($ty),*)
        where
            $($ty: Param,)*
        {
            type Values = maybe_paren!($($ty::Ty),*);

            #[allow(non_snake_case)]
            fn initialize(ctx: &mut FnCtx) -> Self::Values {
                let mut idxs = (0..ctx.builder.block_params(ctx.current_block).len());
                $(
                    let $ty = $ty::initialize_param_at(ctx, &mut idxs);
                )*

                assert!(idxs.next().is_none());

                ($($ty),*)
            }
        }
    };
}

for_all_tuples!(impl_params_tuples);

pub trait FuncRet {
    fn from_func_ret(vals: &[Value]) -> Self;
    fn return_(self, ctx: &mut FnCtx);
}

impl<T> FuncRet for Val<T> {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert_eq!(vals.len(), 1);
        Val::from_value(vals[0])
    }

    fn return_(self, ctx: &mut FnCtx) {
        ctx.builder.ins().return_(&[self.value()]);
    }
}

impl FuncRet for () {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert!(vals.is_empty());
    }

    fn return_(self, ctx: &mut FnCtx) {
        ctx.builder.ins().return_(&[]);
    }
}

pub trait Results: ToAbiParams {
    type Results: FuncRet;
}

impl<T: Primitive + ToAbiParams> Results for T {
    type Results = Val<T>;
}

impl Results for () {
    type Results = ();
}

pub trait Call<I, O> {
    fn fn_call(self, input: I) -> O;
}

impl<F, A, O> Call<(A,), O> for F
where
    F: FnMut(A) -> O,
{
    fn fn_call(mut self, (input,): (A,)) -> O {
        (self)(input)
    }
}

impl<F, A, B, O> Call<(A, B), O> for F
where
    F: FnMut(A, B) -> O,
{
    fn fn_call(mut self, (a, b): (A, B)) -> O {
        (self)(a, b)
    }
}

impl<F, O> Call<(), O> for F
where
    F: FnMut() -> O,
{
    fn fn_call(mut self, _: ()) -> O {
        (self)()
    }
}

impl<F, A, O, I> Call<I, <<Self as HostFn>::Returns as Results>::Results> for HostFunc<F, (A,), O>
where
    Self: HostFn,
    I: IntoParams<Input = <Self as HostFn>::Params>,
{
    fn fn_call(self, p: I) -> <<Self as HostFn>::Returns as Results>::Results {
        self.call(p)
    }
}

impl<F, A, B, O, I> Call<I, <<Self as HostFn>::Returns as Results>::Results>
    for HostFunc<F, (A, B), O>
where
    Self: HostFn,
    I: IntoParams<Input = <Self as HostFn>::Params>,
{
    fn fn_call(self, p: I) -> <<Self as HostFn>::Returns as Results>::Results {
        self.call(p)
    }
}
