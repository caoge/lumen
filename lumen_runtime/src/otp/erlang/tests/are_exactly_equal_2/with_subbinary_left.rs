use super::*;

use proptest::strategy::Strategy;

#[test]
fn without_binary_right_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::binary::sub(arc_process.clone()),
                    strategy::term(arc_process.clone())
                        .prop_filter("Right must not be a binary", |v| !v.is_binary()),
                ),
                |(left, right)| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(left, right), false.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_heap_binary_right_with_same_bytes_returns_true() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &strategy::term::binary::sub::is_binary(arc_process.clone()).prop_map(
                    |subbinary_term| {
                        let subbinary: &sub::Binary = subbinary_term.unbox_reference();
                        let heap_binary_byte_vec: Vec<u8> = subbinary.byte_iter().collect();

                        let heap_binary =
                            Term::slice_to_binary(&heap_binary_byte_vec, &arc_process);
                        (subbinary_term, heap_binary)
                    },
                ),
                |(left, right)| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(left, right), true.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_heap_binary_right_with_different_bytes_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &strategy::term::binary::sub::is_binary::is_not_empty(arc_process.clone())
                    .prop_map(|subbinary_term| {
                        let subbinary: &sub::Binary = subbinary_term.unbox_reference();
                        // same size, but different values by inverting
                        let heap_binary_byte_vec: Vec<u8> =
                            subbinary.byte_iter().map(|b| !b).collect();

                        let heap_binary =
                            Term::slice_to_binary(&heap_binary_byte_vec, &arc_process);
                        (subbinary_term, heap_binary)
                    }),
                |(left, right)| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(left, right), false.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_same_subbinary_right_returns_true() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &strategy::term::binary::sub(arc_process.clone()),
                |operand| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(operand, operand), true.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_same_value_subbinary_right_returns_true() {
    with_process_arc(|arc_process| {
        let original_arc_process = arc_process.clone();
        let subbinary_arc_process = arc_process.clone();

        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::binary::sub::byte_offset(),
                    strategy::term::binary::sub::bit_offset(),
                    strategy::term::binary::sub::byte_count(),
                    strategy::term::binary::sub::bit_count(),
                )
                    .prop_flat_map(move |(byte_offset, bit_offset, byte_count, bit_count)| {
                        let original_bit_len = byte_offset * 8
                            + bit_offset as usize
                            + byte_count * 8
                            + bit_count as usize;
                        let original_byte_len = strategy::bits_to_bytes(original_bit_len);

                        let original = strategy::term::binary::heap::with_size_range(
                            (original_byte_len..=original_byte_len).into(),
                            original_arc_process.clone(),
                        );

                        (
                            Just(byte_offset),
                            Just(bit_offset),
                            Just(byte_count),
                            Just(bit_count),
                            original,
                        )
                    })
                    .prop_map(
                        move |(byte_offset, bit_offset, byte_count, bit_count, original)| {
                            (
                                Term::subbinary(
                                    original,
                                    byte_offset,
                                    bit_offset,
                                    byte_count,
                                    bit_count,
                                    &subbinary_arc_process,
                                ),
                                Term::subbinary(
                                    original,
                                    byte_offset,
                                    bit_offset,
                                    byte_count,
                                    bit_count,
                                    &subbinary_arc_process,
                                ),
                            )
                        },
                    ),
                |(left, right)| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(left, right), true.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}

#[test]
fn with_different_subbinary_right_returns_false() {
    with_process_arc(|arc_process| {
        TestRunner::new(Config::with_source_file(file!()))
            .run(
                &(
                    strategy::term::binary::sub(arc_process.clone()),
                    strategy::term::binary::sub(arc_process.clone()),
                )
                    .prop_filter("Subbinaries must be different", |(left, right)| {
                        left != right
                    }),
                |(left, right)| {
                    prop_assert_eq!(erlang::are_exactly_equal_2(left, right), false.into());

                    Ok(())
                },
            )
            .unwrap();
    });
}