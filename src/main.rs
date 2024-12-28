use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Add;

use cranelift::prelude::*;
use cranelift::prelude::types::{I32, I64};
use cranelift::prelude::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

#[derive(Copy, Clone)]
struct Func<P, R> {
    id: FuncId,
    _pth: PhantomData<fn(P) -> R>,
}

trait ToJitPrimitive {
    fn ty() -> Type;
}

trait ToAbiParams {
    fn to_abi_params(params: &mut Vec<AbiParam>);
}

impl ToAbiParams for () {
    fn to_abi_params(_params: &mut Vec<AbiParam>) { }
}

impl<T: ToJitPrimitive> ToAbiParams for T {
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        params.push(AbiParam::new(T::ty()));
    }
}

macro_rules! primitive_jit_ty {
    ($($src:ident => $dst:ident $(,)?)*) => {
        $(
            impl ToJitPrimitive for $src {
                fn ty() -> Type {
                    $dst
                }
            }
        )*
    };
}

primitive_jit_ty! {
    u32 => I32,
    i32 => I32,
    i64 => I64,
    u64 => I64,
}

impl<A, B> ToAbiParams for (A, B)
where
    A: ToAbiParams,
    B: ToAbiParams,
{
    fn to_abi_params(params: &mut Vec<AbiParam>) {
        A::to_abi_params(params);
        B::to_abi_params(params);
    }
}

struct Context {
    fn_builder_ctx: FunctionBuilderContext,
}

#[derive(Copy, Clone)]
struct Var<T> {
    variable: Variable,
    _pth: PhantomData<T>,
}

struct Val<T> {
    value: Value,
    _pth: PhantomData<T>,
}

trait IntoParams<T> {
    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>);
}

impl<T> IntoParams<Var<T>> for Var<T> {
    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        let val = ctx.builder.use_var(self.variable);
        out.push(val);
    }
}

impl IntoParams<Var<u32>> for Val<u32> {
    fn params(&self, _ctx: &mut FnCtx, out: &mut Vec<Value>) {
        out.push(self.value);
    }
}

impl<A, B, C, D> IntoParams<(Var<C>, Var<D>)> for (A, B) where 
    A: IntoParams<Var<C>>,
    B: IntoParams<Var<D>>,
{
    fn params(&self, ctx: &mut FnCtx, out: &mut Vec<Value>) {
        self.0.params(ctx, out);
        self.1.params(ctx, out);
    }
}

trait Params: ToAbiParams {
    type Values;

    fn initialize(ctx: &mut FnCtx) -> Self::Values;
}

impl Params for () {
    type Values = ();

    fn initialize(_ctx: &mut FnCtx) -> Self::Values { }
}

impl<T: ToJitPrimitive> Params for T {
    type Values = Var<T>;

    fn initialize(ctx: &mut FnCtx) -> Self::Values {
        initialize_param_at::<Self>(ctx, 0)
    }
}

fn initialize_param_at<T: ToJitPrimitive>(ctx: &mut FnCtx, idx: usize) -> Var<T> { 
    let variable = ctx.declare_var();
    let val = ctx.builder.block_params(ctx.current_block)[idx];
    ctx.builder.declare_var(variable, T::ty());
    ctx.builder.def_var(variable, val);

    Var {
        variable,
        _pth: PhantomData,
    }
}

impl<A, B> Params for (A, B)
where 
    A: ToJitPrimitive,
    B: ToJitPrimitive,
{
    type Values = (Var<A>, Var<B>);

    fn initialize(ctx: &mut FnCtx) -> Self::Values {
        (
            initialize_param_at::<A>(ctx, 0),
            initialize_param_at::<B>(ctx, 1),
        ) 
    }
}

trait FromFuncRet {
    fn from_func_ret(vals: &[Value]) -> Self;
}

impl<T> FromFuncRet for Val<T> {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert_eq!(vals.len(), 1);
        Val {
            value: vals[0],
            _pth: PhantomData,
        }
    }
}

impl FromFuncRet for () {
    fn from_func_ret(vals: &[Value]) -> Self {
        assert!(vals.is_empty());
    }
}

trait Results: ToAbiParams {
    type Results: FromFuncRet;

    fn return_(ctx: &mut FnCtx, results: Self::Results);
}

impl Results for u32 {
    type Results = Val<u32>;

    fn return_(ctx: &mut FnCtx, results: Self::Results) {
        ctx.builder.ins().return_(&[results.value]);
    }
    
}

impl Results for () {
    type Results = ();

    fn return_(_ctx: &mut FnCtx, _results: Self::Results) { }
    
}

trait IntAdd {
    fn add(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value;
}

trait UnsignedInteger {}

impl UnsignedInteger for u32 { }
impl UnsignedInteger for u64 { }

impl<T: UnsignedInteger> IntAdd for T {
    fn add(ctx: &mut FnCtx, lhs: Value, rhs: Value) -> Value {
        ctx.builder.ins().uadd_overflow_trap(lhs, rhs, TrapCode::INTEGER_OVERFLOW)
    }
}

impl<T: IntAdd> Add for Var<T> {
    type Output = Val<T>;

    fn add(self, rhs: Self) -> Self::Output {
        with_ctx(|ctx| -> Val<T> {
            let lhs = ctx.builder.use_var(self.variable);
            let rhs = ctx.builder.use_var(rhs.variable);
                
            Val {
                value: T::add(ctx, lhs, rhs),
                _pth: PhantomData,
            }
        })
    }
}

struct Ctx {
    fn_builder_ctx: FunctionBuilderContext,
    module: JITModule,
    ctx: codegen::Context,
    host_fn_map: HashMap<usize, &'static str>,
}

#[derive(Default)]
struct CtxBuilder {
    registered_functions: Vec<NamedHostFn>,
}

struct NamedHostFn {
    name: &'static str,
    ptr: *const u8,
}

macro_rules! host_fns {
    ($($name:expr => $as:ty $(,)?)*) => {
        [
            $(NamedHostFn {
                name: stringify!($name),
                ptr: ($name as $as).to_fn_ptr(),
            }),*
        ]
    };
}


impl CtxBuilder {
    fn register_host_functions(&mut self, f: impl IntoIterator<Item = NamedHostFn>) {
        self.registered_functions.extend(f);
    }

    fn build(self) -> Ctx {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
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
            host_fn_map,
        }

    }
}

impl Ctx {
    fn new() -> Self {
        Self::builder().build()
    }

    fn func<P, R>(&mut self, name: &str, body: impl FnOnce(P::Values) -> R::Results) -> Func<P, R>
    where
        P: Params,
        R: Results,
    {
        Func::new(self, name, body)
    }

    fn get_compiled_function<P, R>(&self, f: Func<P, R>) -> CompiledFunc<P, R> {
        let ptr = self.module.get_finalized_function(f.id);
        CompiledFunc { ptr, _pth: PhantomData }
    }

    fn builder() -> CtxBuilder {
        CtxBuilder::default()
    }
}

struct CompiledFunc<P, R> {
    ptr: *const u8,
    _pth: PhantomData<fn(P) -> R>,
}

impl<P, R> Call for CompiledFunc<P, R>
where
    P: ToJitPrimitive,
    R: ToJitPrimitive,
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

trait Call {
    type Input;
    type Output;

    fn call(&self, input: Self::Input) -> Self::Output;
}

struct FnCtx<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut JITModule,
    host_fn_map: &'a HashMap<usize, &'static str>,
    var_id: u32,
    current_block: Block,
}

impl FnCtx<'_> {
    fn declare_var(&mut self) -> Variable {
        self.var_id += 1;
        Variable::from_u32(self.var_id - 1)
    }
}

thread_local! {
    static FN_CTX: RefCell<Option<*mut FnCtx<'static>>> = const { RefCell::new(None) };
}

fn with_fn_ctx<F, R>(fn_ctx: &mut FnCtx, f: F) -> R
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

fn with_ctx<R>(f: impl FnOnce(&mut FnCtx) -> R) -> R {
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

impl<P, R> Func<P, R>
where
    P: Params,
    R: Results,
{
    pub fn new<B>(ctx: &mut Ctx, name: &str, body: B) -> Self
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
            host_fn_map: &ctx.host_fn_map,
        };

        let params = P::initialize(&mut fn_ctx);

        let ret = with_fn_ctx(&mut fn_ctx, || {
            body(params)
        });

        R::return_(&mut fn_ctx, ret);

        fn_ctx.builder.finalize();

        let func_id = ctx.module.declare_function(name, Linkage::Export, &ctx.ctx.func.signature).unwrap();
        dbg!();
        ctx.module.define_function(func_id, &mut ctx.ctx).unwrap();
        dbg!();
        ctx.module.clear_context(&mut ctx.ctx);
        ctx.module.finalize_definitions().unwrap();

        Self {
            _pth: PhantomData,
            id: func_id,
        }
    }

    pub fn call(&self, params: impl IntoParams<P::Values>) -> R::Results {
        with_ctx(|ctx| {
            let fn_ref = ctx.module.declare_func_in_func(self.id, ctx.builder.func);
            let mut args = Vec::new();
            params.params(ctx, &mut args);
            let call = ctx.builder.ins().call(fn_ref, &args);
            let results = ctx.builder.inst_results(call);
            R::Results::from_func_ret(results)
        })
    }
}

fn add_any<T>(name: &str, ctx: &mut Ctx) -> Func<(T, T), T>
where 
    T: ToJitPrimitive + Results<Results = Val<T>> + IntAdd
{
    ctx.func::<(T, T), T>(name, |(x, y)| {
        x + y
    })
}

trait HostFn {
    type Input: Params;
    type Output: Results;

    fn to_fn_ptr(self) -> *const u8;
}

impl<A, B> HostFn for extern "C" fn(A) -> B
where
    A: ToJitPrimitive,
    B: Results,
{
    type Input = A;
    type Output = B;

    fn to_fn_ptr(self) -> *const u8 {
        self as *const u8
    }
}

impl HostFn for extern "C" fn()
{
    type Input = ();
    type Output = ();

    fn to_fn_ptr(self) -> *const u8 {
        self as *const u8
    }
}

struct HostFunc<P, R> {
    _pth: PhantomData<fn(P) -> R>
}

fn host_fn<F: HostFn>(f: F) ->  Func<F::Input, F::Output> {
    with_ctx(|ctx| {
        let p = f.to_fn_ptr() as usize;
        let name = ctx.host_fn_map.get(&p).unwrap();
        let mut sig = ctx.module.make_signature();
        <F::Input as ToAbiParams>::to_abi_params(&mut sig.params);
        <F::Output as ToAbiParams>::to_abi_params(&mut sig.returns);
        let id = ctx.module.declare_function(name, Linkage::Import, &sig).unwrap();
        Func {
            id,
            _pth: PhantomData,
        }
    })
}

extern "C" fn print_hello<T: Display>(t: T) {
    println!("hello world: {t}");
}

fn main() {
    let mut builder = Ctx::builder();
    builder.register_host_functions(host_fns![
        print_hello::<u32> => extern "C" fn(u32),
    ]);

    let mut ctx = builder.build();

    let add = add_any::<u32>("add", &mut ctx);

    let main = ctx.func::<(u32, u32), u32>("main", |(x, y)| {
        let f = host_fn(print_hello::<u32> as extern "C" fn(u32));
        f.call(x);
        add.call((x, y))
    });

    let main = ctx.get_compiled_function(main);

    dbg!(main.call((1, 3)));
}
