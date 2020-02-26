use std::collections::HashSet;
use std::ffi::CString;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::mem;

use libeir_intern::{Ident, Symbol};
use libeir_ir::FunctionIdent;

use liblumen_core::symbols::FunctionSymbol;

use crate::codegen::CompiledModule;
use crate::llvm::*;
use crate::Result;

/// Generates an LLVM module containing the raw symbol table data for the current build
///
/// This is similar to the atom table generation, but simpler, in that we just generate
/// a large list of `FunctionSybmol` structs, which reference extern declarations of all
/// the functions defined by the build. At link time these will be resolved to pointers
/// to the actual functions, and when we boot the runtime, we can reify this array into
/// a more efficient search structure for dispatch.
pub fn compile_symbol_table(
    context: &Context,
    target_machine: &TargetMachine,
    symbols: HashSet<FunctionSymbol>,
    output_dir: &Path,
) -> Result<Arc<CompiledModule>> {
    const NAME: &'static str = "liblumen_crt_dispatch";

    let builder = ModuleBuilder::new(NAME, context, target_machine)?;

    fn declare_extern_symbol<'ctx>(
        builder: &ModuleBuilder<'ctx>,
        symbol: &FunctionSymbol,
    ) -> Result<LLVMValueRef> {
        let ms = unsafe { mem::transmute::<u32, Symbol>(symbol.module as u32) };
        let fs = unsafe { mem::transmute::<u32, Symbol>(symbol.function as u32) };
        let ident = FunctionIdent {
            module: Ident::with_empty_span(ms),
            name: Ident::with_empty_span(fs),
            arity: symbol.arity as usize,
        };
        let name = CString::new(ident.to_string()).unwrap();
        let ty = builder.get_erlang_function_type(ident.arity);
        Ok(builder.add_function(&name, ty))
    }

    // Translate FunctionIdent to FunctionSymbol with pointer to declared function
    let usize_type = builder.get_usize_type();
    let i8_type = builder.get_i8_type();
    let fn_ptr_type = builder.get_pointer_type(builder.get_opaque_function_type());
    let function_type = builder.get_struct_type(
        Some("FunctionSymbol"),
        &[usize_type, usize_type, i8_type, fn_ptr_type],
    );

    // Build values for array
    let mut functions = Vec::with_capacity(symbols.len());
    for symbol in symbols.iter() {
        let decl = declare_extern_symbol(&builder, symbol)?;
        let module = builder.build_constant_uint(usize_type, symbol.module);
        let fun = builder.build_constant_uint(usize_type, symbol.function);
        let arity = builder.build_constant_uint(i8_type, symbol.arity as usize);
        let function = builder.build_constant_struct(function_type, &[module, fun, arity, decl]);
        functions.push(function);
    }

    // Generate global array of all idents
    let functions_const = builder.build_constant_array(function_type, functions.as_slice());
    builder.set_linkage(functions_const, Linkage::Private);
    builder.set_alignment(functions_const, 8);

    let function_ptr_type = builder.get_pointer_type(function_type);
    let table_global_init = builder.build_const_inbounds_gep(functions_const, &[0, 0]);
    let table_global = builder.add_global(
        function_ptr_type,
        "__LUMEN_SYMBOL_TABLE",
        Some(table_global_init),
    );
    builder.set_alignment(table_global, 8);

    // Generate array length global
    let table_size_global_init = builder.build_constant_uint(usize_type, functions.len());
    let table_size_global = builder.add_global(
        usize_type,
        "__LUMEN_SYMBOL_TABLE_SIZE",
        Some(table_size_global_init),
    );
    builder.set_alignment(table_size_global, 8);

    // Finalize module
    let module = builder.finish();
    // Open object file for writing
    let path = output_dir.join(&format!("{}.o", NAME));
    let mut file = File::create(path.as_path())?;
    // Emit object file
    module.emit_obj(&mut file)?;

    Ok(Arc::new(CompiledModule::new(
        NAME.to_string(),
        Some(path),
        None,
    )))
}