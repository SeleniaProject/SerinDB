#[cfg(feature = "jit")]
use cranelift_codegen::ir::types;
#[cfg(feature = "jit")]
use cranelift_codegen::{Context, settings};
#[cfg(feature = "jit")]
use cranelift_jit::{JITBuilder, JITModule};
#[cfg(feature = "jit")]
use cranelift_module::{FuncId, Linkage, Module};

/// JIT engine wrapper.
#[cfg(feature = "jit")]
pub struct JitEngine {
    module: JITModule,
}

#[cfg(feature = "jit")]
impl JitEngine {
    /// Create new engine.
    pub fn new() -> Self {
        let builder = JITBuilder::with_builder(settings::builder());
        let module = JITModule::new(builder);
        Self { module }
    }

    /// Compile constant returning function `fn() -> i64`.
    pub fn compile_const(&mut self, value: i64) -> *const u8 {
        let mut ctx = self.module.make_context();
        let sig = self.module.make_signature();
        ctx.func.signature = sig;
        use cranelift_codegen::ir::AbiParam;
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
        let mut fb_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let block = builder.create_block();
        builder.switch_to_block(block);
        builder.seal_block(block);
        let val = builder.ins().iconst(types::I64, value);
        builder.ins().return_(&[val]);
        builder.finalize();
        let id: FuncId = self
            .module
            .declare_function("const_fn", Linkage::Export, &ctx.func.signature)
            .unwrap();
        self.module.define_function(id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions();
        self.module.get_finalized_function(id)
    }
}

#[cfg(test)]
#[cfg(feature = "jit")]
mod tests {
    use super::*;

    #[test]
    fn jit_constant() {
        let mut eng = JitEngine::new();
        let func = eng.compile_const(42);
        let compiled: extern "C" fn() -> i64 = unsafe { std::mem::transmute(func) };
        assert_eq!(compiled(), 42);
    }
} 