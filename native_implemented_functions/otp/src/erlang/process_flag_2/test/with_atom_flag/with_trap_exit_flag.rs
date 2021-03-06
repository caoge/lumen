use super::*;

use liblumen_alloc::erts::process::code::stack::frame::Placement;
use liblumen_alloc::erts::term::prelude::{Atom, Term};

use lumen_rt_full::scheduler::Scheduler;

use crate::erlang;
use crate::test::{self, has_message, has_no_message};

#[test]
fn without_boolean_value_errors_badarg() {
    run!(
        |arc_process| {
            (
                Just(arc_process.clone()),
                strategy::term::is_not_boolean(arc_process.clone()),
            )
        },
        |(arc_process, value)| {
            prop_assert_is_not_boolean!(
                native(&arc_process, flag(), value),
                "trap_exit value",
                value
            );

            Ok(())
        },
    );
}

#[test]
fn with_boolean_returns_original_value_false() {
    TestRunner::new(Config::with_source_file(file!()))
        .run(&strategy::term::is_boolean(), |value| {
            let arc_process = test::process::default();

            prop_assert_eq!(native(&arc_process, flag(), value), Ok(false.into()));

            Ok(())
        })
        .unwrap();
}

#[test]
fn with_true_value_then_boolean_value_returns_old_value_true() {
    TestRunner::new(Config::with_source_file(file!()))
        .run(&strategy::term::is_boolean(), |value| {
            let arc_process = test::process::default();

            let old_value = true.into();
            prop_assert_eq!(native(&arc_process, flag(), old_value), Ok(false.into()));

            prop_assert_eq!(native(&arc_process, flag(), value), Ok(old_value));

            Ok(())
        })
        .unwrap();
}

#[test]
fn with_true_value_with_linked_and_does_not_exit_when_linked_process_exits_normal() {
    with_process(|process| {
        let other_arc_process = test::process::child(process);

        process.link(&other_arc_process);

        assert_eq!(native(process, flag(), true.into()), Ok(false.into()));

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(!other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        let reason = Atom::str_to_term("normal");

        erlang::exit_1::place_frame_with_arguments(&other_arc_process, Placement::Replace, reason)
            .unwrap();

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(other_arc_process.is_exiting());
        assert!(!process.is_exiting());
        assert!(has_no_message(process));
    });
}

#[test]
fn with_true_value_with_linked_and_does_not_exit_when_linked_process_exits_shutdown() {
    with_process(|process| {
        let other_arc_process = test::process::child(process);

        process.link(&other_arc_process);

        assert_eq!(native(process, flag(), true.into()), Ok(false.into()));

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(!other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        let reason = Atom::str_to_term("shutdown");

        erlang::exit_1::place_frame_with_arguments(&other_arc_process, Placement::Replace, reason)
            .unwrap();

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(other_arc_process.is_exiting());
        assert!(!process.is_exiting());
        assert!(has_no_message(process));
    });
}

#[test]
fn with_true_value_with_linked_and_does_not_exit_when_linked_process_exits_with_shutdown_tuple() {
    with_process(|process| {
        let other_arc_process = test::process::child(process);

        process.link(&other_arc_process);

        assert_eq!(native(process, flag(), true.into()), Ok(false.into()));

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(!other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        let tag = Atom::str_to_term("shutdown");
        let shutdown_reason = Atom::str_to_term("shutdown_reason");
        let reason = other_arc_process
            .tuple_from_slice(&[tag, shutdown_reason])
            .unwrap();

        erlang::exit_1::place_frame_with_arguments(&other_arc_process, Placement::Replace, reason)
            .unwrap();

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(other_arc_process.is_exiting());
        assert!(!process.is_exiting());
        assert!(has_no_message(process));
    });
}

#[test]
fn with_true_value_with_linked_receive_exit_message_and_does_not_exit_when_linked_process_exits() {
    with_process(|process| {
        let other_arc_process = test::process::child(process);

        process.link(&other_arc_process);

        assert_eq!(native(process, flag(), true.into()), Ok(false.into()));

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(!other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        let reason = Atom::str_to_term("exit_reason");

        erlang::exit_1::place_frame_with_arguments(&other_arc_process, Placement::Replace, reason)
            .unwrap();

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        let tag = Atom::str_to_term("EXIT");
        let from = other_arc_process.pid_term();
        let exit_message = process.tuple_from_slice(&[tag, from, reason]).unwrap();

        assert_has_message!(process, exit_message);
    });
}

#[test]
fn with_true_value_then_false_value_exits_when_linked_process_exits() {
    with_process(|process| {
        let other_arc_process = test::process::child(process);

        process.link(&other_arc_process);

        assert_eq!(native(process, flag(), true.into()), Ok(false.into()));

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(!other_arc_process.is_exiting());
        assert!(!process.is_exiting());

        assert_eq!(native(process, flag(), false.into()), Ok(true.into()));

        let reason = Atom::str_to_term("exit_reason");

        erlang::exit_1::place_frame_with_arguments(&other_arc_process, Placement::Replace, reason)
            .unwrap();

        assert!(Scheduler::current().run_through(&other_arc_process));

        assert!(other_arc_process.is_exiting());
        assert!(process.is_exiting());
    });
}

fn flag() -> Term {
    Atom::str_to_term("trap_exit")
}
