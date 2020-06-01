// wasm32 proptest cannot be compiled at the same time as non-wasm32 proptest, so disable tests that
// use proptest completely for wasm32
//
// See https://github.com/rust-lang/cargo/issues/4866
#[cfg(all(not(target_arch = "wasm32"), test))]
mod test;

use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::Term;



use crate::erlang::spawn_apply_3;
use crate::runtime::process::spawn::options::Options;

#[native_implemented::function(spawn_monitor/3)]
pub fn result(
    process: &Process,
    module: Term,
    function: Term,
    arguments: Term,
) -> exception::Result<Term> {
    let options = Options {
        monitor: true,
        ..Default::default()
    };

    spawn_apply_3::result(process, options, module, function, arguments)
}
