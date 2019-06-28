use super::*;

use crate::process::local::pid_to_process;
use crate::process::{ModuleFunctionArity, Status};

mod with_atom_module;

#[test]
fn without_atom_module_errors_badarg() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::is_not_atom(arc_process.clone()),
                    strategy::term::atom(),
                    strategy::term::list::proper(arc_process.clone()),
                ),
                |(module, function, arguments)| {
                    prop_assert_eq!(
                        erlang::spawn_3(module, function, arguments, &arc_process),
                        Err(badarg!())
                    );

                    Ok(())
                },
            )
            .unwrap();
    });
}