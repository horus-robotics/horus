use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_native;
use std::collections::HashMap;
use std::mem;

/// JIT compiler for ultra-fast node execution
/// Compiles deterministic nodes to native code for 20-50ns execution
pub struct JITCompiler {
    /// The JIT module
    module: JITModule,
    /// Context for code generation
    ctx: codegen::Context,
    /// Function builder context
    func_ctx: FunctionBuilderContext,
    /// Compiled function IDs
    compiled_funcs: HashMap<String, FuncId>,
}

impl JITCompiler {
    /// Create new JIT compiler
    pub fn new() -> Result<Self, String> {
        // Get native target
        let isa = cranelift_native::builder()
            .map_err(|e| format!("Failed to create ISA builder: {}", e))?
            .finish(settings::Flags::new(settings::builder()))
            .map_err(|e| format!("Failed to create ISA: {}", e))?;

        // Create JIT builder
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Create module
        let module = JITModule::new(builder);

        // Create contexts
        let ctx = module.make_context();
        let func_ctx = FunctionBuilderContext::new();

        Ok(Self {
            module,
            ctx,
            func_ctx,
            compiled_funcs: HashMap::new(),
        })
    }

    /// Compile a simple arithmetic dataflow node
    /// This demonstrates compiling a node that does: output = input * 2 + offset
    pub fn compile_arithmetic_node(
        &mut self,
        name: &str,
        multiply_factor: i64,
        offset: i64,
    ) -> Result<*const u8, String> {
        // Clear the context for a fresh function
        self.ctx.clear();

        // Define function signature: fn(input: i64) -> i64
        let int_type = types::I64;
        self.ctx.func.signature.params.push(AbiParam::new(int_type));
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(int_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        {
            // Build the function body
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get the input parameter
            let input = builder.block_params(entry_block)[0];

            // Perform computation: result = input * multiply_factor + offset
            let factor = builder.ins().iconst(int_type, multiply_factor);
            let multiplied = builder.ins().imul(input, factor);
            let offset_val = builder.ins().iconst(int_type, offset);
            let result = builder.ins().iadd(multiplied, offset_val);

            // Return the result
            builder.ins().return_(&[result]);

            // Finalize
            builder.seal_all_blocks();
            builder.finalize();
        }

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        // Clear the context to free resources
        self.module.clear_context(&mut self.ctx);

        // Compile the function
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Store function ID
        self.compiled_funcs.insert(name.to_string(), func_id);

        Ok(code_ptr)
    }

    /// Compile a more complex dataflow with multiple operations
    /// Computes: output = (a + b) * (c - d)
    pub fn compile_dataflow_combiner(&mut self, name: &str) -> Result<*const u8, String> {
        // Clear the context for a fresh function
        self.ctx.clear();

        // Define function signature: fn(a: i64, b: i64, c: i64, d: i64) -> i64
        let int_type = types::I64;
        for _ in 0..4 {
            self.ctx.func.signature.params.push(AbiParam::new(int_type));
        }
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(int_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        {
            // Build the function body
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Get parameters
            let params = builder.block_params(entry_block);
            let a = params[0];
            let b = params[1];
            let c = params[2];
            let d = params[3];

            // Compute: (a + b) * (c - d)
            let sum = builder.ins().iadd(a, b);
            let diff = builder.ins().isub(c, d);
            let result = builder.ins().imul(sum, diff);

            // Return the result
            builder.ins().return_(&[result]);

            // Finalize
            builder.seal_all_blocks();
            builder.finalize();
        }

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        // Clear the context to free resources
        self.module.clear_context(&mut self.ctx);

        // Compile the function
        self.module
            .finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Store function ID
        self.compiled_funcs.insert(name.to_string(), func_id);

        Ok(code_ptr)
    }

    /// Execute a compiled arithmetic function
    ///
    /// # Safety
    /// The caller must ensure that `func_ptr` points to valid JIT-compiled code
    /// that was generated by this compiler with the correct signature `fn(i64) -> i64`.
    pub unsafe fn execute_arithmetic(&self, func_ptr: *const u8, input: i64) -> i64 {
        // Cast to function pointer
        let func: fn(i64) -> i64 = mem::transmute(func_ptr);
        func(input)
    }

    /// Execute a compiled dataflow combiner
    ///
    /// # Safety
    /// The caller must ensure that `func_ptr` points to valid JIT-compiled code
    /// that was generated by this compiler with the correct signature `fn(i64, i64, i64, i64) -> i64`.
    pub unsafe fn execute_combiner(
        &self,
        func_ptr: *const u8,
        a: i64,
        b: i64,
        c: i64,
        d: i64,
    ) -> i64 {
        // Cast to function pointer
        let func: fn(i64, i64, i64, i64) -> i64 = mem::transmute(func_ptr);
        func(a, b, c, d)
    }
}
