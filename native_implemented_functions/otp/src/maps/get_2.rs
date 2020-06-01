// wasm32 proptest cannot be compiled at the same time as non-wasm32 proptest, so disable tests that
// use proptest completely for wasm32
//
// See https://github.com/rust-lang/cargo/issues/4866
#[cfg(all(not(target_arch = "wasm32"), test))]
mod test;

use anyhow::*;

use liblumen_alloc::erts::exception::{self, *};
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;



#[native_implemented::function(get/2)]
pub fn result(process: &Process, key: Term, map: Term) -> exception::Result<Term> {
    let boxed_map = term_try_into_map_or_badmap!(process, map)?;

    match boxed_map.get(key) {
        Some(value) => Ok(value),
        None => Err(badkey(
            process,
            key,
            anyhow!("map ({}) does not have key ({})", map, key).into(),
        )),
    }
}
