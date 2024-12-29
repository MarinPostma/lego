use std::collections::BTreeMap;

use cranelift::prelude::{EntityRef, InstBuilder, MemFlags, StackSlotKind};
use cranelift::prelude::Value;
use cranelift_codegen::ir::{DynamicStackSlotData, DynamicType, StackSlot};
use cranelift::prelude::types::I64;

use crate::func::{with_ctx, FnCtx};

pub struct Struct<L> {
    loader: L,
    layout: StructLayout,
}

struct StackLoader {
    slot: StackSlot, }

impl Loader for StackLoader {
    fn load(&self, ctx: &mut FnCtx, offset: i32, ty: Type) -> Value {
        ctx.builder.ins().stack_load(I64, self.slot, offset)
    }

    fn store(&self, ctx: &mut FnCtx, offset: i32, val: Value) {
        ctx.builder.ins().stack_store(x, SS, Offset)
    }
}

trait Loader {
    fn load(&self, ctx: &mut FnCtx, offset: i32, ty: Type) -> Value;
    fn store(&self, ctx: &mut FnCtx, offset: i32, val: Value);
}

pub struct StructLayout {
    fields: BTreeMap<String, StructField>,
}

pub struct StructField {
    offset: i32,
    ty: Type,
}

enum Type {
    Primitive(PrimitiveType),
    Struct(StructLayout),
}

enum PrimitiveType {

}
