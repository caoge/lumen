#[cfg(test)]
mod test;

use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;

use lumen_rt_core::time::{monotonic, Unit::Native};

use native_implemented_function::native_implemented_function;

#[native_implemented_function(monotonic_time/0)]
pub fn native(process: &Process) -> exception::Result<Term> {
    let big_int = monotonic::time(Native);

    Ok(process.integer(big_int)?)
}
