// wasm32 proptest cannot be compiled at the same time as non-wasm32 proptest, so disable tests that
// use proptest completely for wasm32
//
// See https://github.com/rust-lang/cargo/issues/4866
#[cfg(all(not(target_arch = "wasm32"), test))]
mod test;

use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;

use native_implemented_function::native_implemented_function;

#[native_implemented_function(tuple_size/1)]
pub fn native(process: &Process, tuple: Term) -> exception::Result<Term> {
    let tuple = term_try_into_tuple!(tuple)?;
    let size = process.integer(tuple.len())?;

    Ok(size)
}
