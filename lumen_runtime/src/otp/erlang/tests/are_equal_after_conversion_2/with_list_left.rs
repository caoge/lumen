use super::*;

use proptest::collection::SizeRange;
use proptest::strategy::Strategy;

use crate::otp::erlang::tests::strategy::NON_EMPTY_RANGE_INCLUSIVE;

#[test]
fn without_list_right_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::list::non_empty_maybe_improper(arc_process.clone()),
                    strategy::term(arc_process.clone())
                        .prop_filter("Right must not be list", |v| !v.is_list()),
                ),
                |(left, right)| {
                    prop_assert_eq!(
                        erlang::are_equal_after_conversion_2(left, right),
                        false.into()
                    );

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_same_list_right_returns_true() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &strategy::term::list::non_empty_maybe_improper(arc_process.clone()),
                |operand| {
                    prop_assert_eq!(
                        erlang::are_equal_after_conversion_2(operand, operand),
                        true.into()
                    );

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_same_value_list_right_returns_true() {
    let size_range: SizeRange = NON_EMPTY_RANGE_INCLUSIVE.clone().into();

    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &proptest::collection::vec(strategy::term(arc_process.clone()), size_range)
                    .prop_map(move |vec| match vec.len() {
                        1 => (
                            Term::slice_to_list(&vec, &arc_process),
                            Term::slice_to_list(&vec, &arc_process),
                        ),
                        len => {
                            let last_index = len - 1;

                            (
                                Term::slice_to_improper_list(
                                    &vec[0..last_index],
                                    vec[last_index],
                                    &arc_process,
                                ),
                                Term::slice_to_improper_list(
                                    &vec[0..last_index],
                                    vec[last_index],
                                    &arc_process,
                                ),
                            )
                        }
                    }),
                |(left, right)| {
                    prop_assert_eq!(
                        erlang::are_equal_after_conversion_2(left, right),
                        true.into()
                    );

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_different_list_right_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::list::non_empty_maybe_improper(arc_process.clone()),
                    strategy::term::list::non_empty_maybe_improper(arc_process.clone()),
                )
                    .prop_filter("Lists must be different", |(left, right)| left != right),
                |(left, right)| {
                    prop_assert_eq!(
                        erlang::are_equal_after_conversion_2(left, right),
                        false.into()
                    );

                    Ok(())
                },
            )
            .unwrap();
    });
}