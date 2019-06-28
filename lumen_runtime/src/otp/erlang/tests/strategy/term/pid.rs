use std::sync::Arc;

use proptest::strategy::{BoxedStrategy, Strategy};

use crate::process::Process;
use crate::term::Term;

pub mod external;

pub fn external(arc_process: Arc<Process>) -> BoxedStrategy<Term> {
    let external_pid_arc_process = arc_process.clone();

    (external::node(), number(), serial())
        .prop_map(move |(node, number, serial)| {
            Term::external_pid(node, number, serial, &external_pid_arc_process.clone()).unwrap()
        })
        .boxed()
}

pub fn local() -> BoxedStrategy<Term> {
    (number(), serial())
        .prop_map(|(number, serial)| Term::local_pid(number, serial).unwrap())
        .boxed()
}

pub fn number() -> BoxedStrategy<usize> {
    (0..=crate::process::identifier::NUMBER_MAX).boxed()
}

pub fn serial() -> BoxedStrategy<usize> {
    (0..=crate::process::identifier::SERIAL_MAX).boxed()
}