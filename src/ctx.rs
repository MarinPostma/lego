use std::collections::HashMap;
use std::marker::PhantomData;

use cranelift::prelude::Configurable as _;
use cranelift_codegen::settings;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Module;

use crate::func::{CompiledFunc, Func, Params, Results};

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
        // flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("opt_level", "none").unwrap();
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

        let mut ctx = module.make_context();
        ctx.want_disasm = true;
        Ctx {
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx,
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

    pub fn func<P, R>(&mut self, body: impl FnOnce(P::Values) -> R::Results) -> Func<P, R>
    where
        P: Params,
        R: Results,
    {
        Func::new(self, body)
    }

    pub fn get_compiled_function<P, R>(&self, f: Func<P, R>) -> CompiledFunc<P, R> {
        let ptr = self.module.get_finalized_function(f.id());
        CompiledFunc {
            ptr,
            _pth: PhantomData,
        }
    }

    // #[cfg(target_arch = "x86_64")]
    // pub fn disas<P, R>(&self, f: Func<P, R>) {
    //     use iced_x86::{Decoder, DecoderOptions, Formatter as _, Instruction, NasmFormatter};
    //     const HEXBYTES_COLUMN_BYTE_LENGTH: usize = 10;
    //     const EXAMPLE_CODE_BITNESS: u32 = 64;
    //     const EXAMPLE_CODE_RIP: u64 = 0x0000_7FFA_C46A_CDA4;
    //
    //     let bytes = self.module.get_finalized_function_bytes(f.id());
    //     let mut decoder =
    //     Decoder::with_ip(EXAMPLE_CODE_BITNESS, bytes, EXAMPLE_CODE_RIP, DecoderOptions::NONE);
    //
    //     // Formatters: Masm*, Nasm*, Gas* (AT&T) and Intel* (XED).
    //     // For fastest code, see `SpecializedFormatter` which is ~3.3x faster. Use it if formatting
    //     // speed is more important than being able to re-assemble formatted instructions.
    //     let mut formatter = NasmFormatter::new();
    //
    //     // Change some options, there are many more
    //     formatter.options_mut().set_digit_separator("`");
    //     formatter.options_mut().set_first_operand_char_index(10);
    //
    //     // String implements FormatterOutput
    //     let mut output = String::new();
    //
    //     // Initialize this outside the loop because decode_out() writes to every field
    //     let mut instruction = Instruction::default();
    //
    //     // The decoder also implements Iterator/IntoIterator so you could use a for loop:
    //     //      for instruction in &mut decoder { /* ... */ }
    //     // or collect():
    //     //      let instructions: Vec<_> = decoder.into_iter().collect();
    //     // but can_decode()/decode_out() is a little faster:
    //     while decoder.can_decode() {
    //         // There's also a decode() method that returns an instruction but that also
    //         // means it copies an instruction (40 bytes):
    //         //     instruction = decoder.decode();
    //         decoder.decode_out(&mut instruction);
    //
    //         // Format the instruction ("disassemble" it)
    //         output.clear();
    //         formatter.format(&instruction, &mut output);
    //
    //         // Eg. "00007FFAC46ACDB2 488DAC2400FFFFFF     lea       rbp,[rsp-100h]"
    //         print!("{:016X} ", instruction.ip());
    //         let start_index = (instruction.ip() - EXAMPLE_CODE_RIP) as usize;
    //         let instr_bytes = &bytes[start_index..start_index + instruction.len()];
    //         for b in instr_bytes.iter() {
    //             print!("{:02X}", b);
    //         }
    //         if instr_bytes.len() < HEXBYTES_COLUMN_BYTE_LENGTH {
    //             for _ in 0..HEXBYTES_COLUMN_BYTE_LENGTH - instr_bytes.len() {
    //                 print!("  ");
    //             }
    //         }
    //         println!(" {}", output);
    //     }
    // }
    //
    // #[cfg(target_arch = "aarch64")]
    // pub fn disas<P, R>(&self, f: Func<P, R>) {
    //     let bytes = self.module.get_finalized_function_bytes(f.id());
    //     let code = unsafe { std::mem::transmute::<&[u8], &[u32]>(bytes) };
    //     dbg!(code.len());
    //     for ins in code {
    //         let insn = disarm64::decoder::decode(*ins).unwrap();
    //         println!("{insn}");
    //     }
    // }

    pub fn builder() -> CtxBuilder {
        CtxBuilder::default()
    }

    pub fn ctx(&self) -> &cranelift::prelude::codegen::Context {
        &self.ctx
    }
}
