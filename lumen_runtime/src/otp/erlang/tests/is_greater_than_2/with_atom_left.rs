use super::*;

use proptest::strategy::Strategy;

#[test]
fn with_number_returns_true() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::atom(),
                    strategy::term::is_number(arc_process.clone()),
                ),
                |(left, right)| {
                    prop_assert_eq!(erlang::is_greater_than_2(left, right), true.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_greater_atom_returns_true() {
    is_greater_than(|_, _| Term::str_to_atom("keft", DoNotCare).unwrap(), true);
}

#[test]
fn with_same_atom_returns_false() {
    is_greater_than(|left, _| left, false);
}

#[test]
fn with_same_atom_value_returns_false() {
    is_greater_than(|_, _| Term::str_to_atom("left", DoNotCare).unwrap(), false);
}

#[test]
fn with_greater_atom_returns_false() {
    is_greater_than(|_, _| Term::str_to_atom("meft", DoNotCare).unwrap(), false);
}

#[test]
fn without_number_or_atom_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::atom(),
                    strategy::term(arc_process.clone())
                        .prop_filter("Right cannot be a number or atom", |right| {
                            !(right.is_atom() || right.is_number())
                        }),
                ),
                |(left, right)| {
                    prop_assert_eq!(erlang::is_greater_than_2(left, right), false.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

fn is_greater_than<R>(right: R, expected: bool)
where
    R: FnOnce(Term, &Process) -> Term,
{
    super::is_greater_than(
        |_| Term::str_to_atom("left", DoNotCare).unwrap(),
        right,
        expected,
    );
}