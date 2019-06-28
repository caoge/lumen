use super::*;

use proptest::strategy::Strategy;

#[test]
fn with_number_atom_reference_function_port_pid_tuple_map_or_list_returns_second() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::binary::heap(arc_process.clone()),
                    strategy::term(arc_process.clone()).prop_filter(
                        "second must be number, atom, reference, function, port, pid, tuple, map, or list",
                        |second| {
                            second.is_number()
                                || second.is_atom()
                                || second.is_reference()
                                || second.is_function()
                                || second.is_port()
                                || second.is_pid()
                                || second.is_tuple()
                                || second.is_list()
                        }),
                ),
                |(first, second)| {
                    prop_assert_eq!(erlang::min_2(first, second), second);

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_prefix_heap_binary_second_returns_second() {
    min(|_, process| Term::slice_to_binary(&[1], &process), Second);
}

#[test]
fn with_same_length_heap_binary_with_lesser_byte_second_returns_second() {
    min(|_, process| Term::slice_to_binary(&[0], &process), Second);
}

#[test]
fn with_longer_heap_binary_with_lesser_byte_second_returns_second() {
    min(
        |_, process| Term::slice_to_binary(&[0, 1, 2], &process),
        Second,
    );
}

#[test]
fn with_same_heap_binary_second_returns_first() {
    min(|first, _| first, First);
}

#[test]
fn with_same_value_heap_binary_second_returns_first() {
    min(|_, process| Term::slice_to_binary(&[1, 1], &process), First)
}

#[test]
fn with_shorter_heap_binary_with_greater_byte_second_returns_first() {
    min(|_, process| Term::slice_to_binary(&[2], &process), First);
}

#[test]
fn with_heap_binary_with_greater_byte_second_returns_first() {
    min(|_, process| Term::slice_to_binary(&[2, 1], &process), First);
}

#[test]
fn with_heap_binary_with_different_greater_byte_second_returns_first() {
    min(|_, process| Term::slice_to_binary(&[1, 2], &process), First);
}

#[test]
fn with_prefix_subbinary_second_returns_second() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[1], &process);
            Term::subbinary(original, 0, 0, 1, 0, &process)
        },
        Second,
    );
}

#[test]
fn with_same_length_subbinary_with_lesser_byte_second_returns_second() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[0, 1], &process);
            Term::subbinary(original, 0, 0, 2, 0, &process)
        },
        Second,
    );
}

#[test]
fn with_longer_subbinary_with_lesser_byte_second_returns_second() {
    min(|_, process| bitstring!(0, 1, 0b10 :: 2, &process), Second);
}

#[test]
fn with_same_subbinary_second_returns_first() {
    min(|first, _| first, First);
}

#[test]
fn with_same_value_subbinary_second_returns_first() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[1, 1], &process);
            Term::subbinary(original, 0, 0, 2, 0, &process)
        },
        First,
    )
}

#[test]
fn with_shorter_subbinary_with_greater_byte_second_returns_first() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[2], &process);
            Term::subbinary(original, 0, 0, 1, 0, &process)
        },
        First,
    );
}

#[test]
fn with_subbinary_with_greater_byte_second_returns_first() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[2, 1], &process);
            Term::subbinary(original, 0, 0, 2, 0, &process)
        },
        First,
    );
}

#[test]
fn with_subbinary_with_different_greater_byte_second_returns_first() {
    min(
        |_, process| {
            let original = Term::slice_to_binary(&[1, 2], &process);
            Term::subbinary(original, 0, 0, 2, 0, &process)
        },
        First,
    );
}

#[test]
fn with_subbinary_with_value_with_shorter_length_returns_first() {
    min(|_, process| bitstring!(1, 1 :: 1, &process), First)
}

fn min<R>(second: R, which: FirstSecond)
where
    R: FnOnce(Term, &Process) -> Term,
{
    super::min(
        |process| Term::slice_to_binary(&[1, 1], &process),
        second,
        which,
    );
}