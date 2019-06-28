//! Mirrors [erlang](http://erlang::org/doc/man/erlang::html) module

use std::cmp::Ordering;
use std::convert::TryInto;
use std::num::FpCategory;
use std::sync::Arc;

use num_bigint::BigInt;
use num_traits::Zero;

use crate::atom::{Existence, Existence::*};
use crate::binary::{heap, sub, Part, ToTerm, ToTermOptions};
use crate::code;
use crate::exception::{Class, Result};
use crate::float::{self, Float};
use crate::integer::{big, small};
use crate::list::Cons;
use crate::map::Map;
use crate::node;
use crate::otp;
use crate::process::local::pid_to_self_or_process;
use crate::process::{IntoProcess, Process, TryIntoInProcess};
use crate::reference;
use crate::registry::{self, Registered};
use crate::scheduler::Scheduler;
use crate::send::{self, send, Sent};
use crate::stacktrace;
use crate::term::{Tag, Tag::*, Term};
use crate::time::{
    self,
    monotonic::{self, Milliseconds},
    Unit::*,
};
use crate::timer::start::ReferenceFrame;
use crate::timer::{self, Timeout};
use crate::tuple::{Tuple, ZeroBasedIndex};

// wasm32 proptest cannot be compiled at the same time as non-wasm32 proptest, so disable tests that
// use proptest completely for wasm32
//
// See https://github.com/rust-lang/cargo/issues/4866
#[cfg(all(not(target_arch = "wasm32"), test))]
mod tests;

pub fn abs_1(number: Term, process: &Process) -> Result {
    let option_abs = match number.tag() {
        SmallInteger => {
            if unsafe { number.small_integer_is_negative() } {
                // cast first so that sign bit is extended on shift
                let signed = (number.tagged as isize) >> Tag::SMALL_INTEGER_BIT_COUNT;
                let positive = -signed;
                Some(Term {
                    tagged: ((positive << Tag::SMALL_INTEGER_BIT_COUNT) as usize)
                        | (SmallInteger as usize),
                })
            } else {
                Some(Term {
                    tagged: number.tagged,
                })
            }
        }
        Boxed => {
            let unboxed: &Term = number.unbox_reference();

            match unboxed.tag() {
                BigInteger => {
                    let big_integer: &big::Integer = number.unbox_reference();
                    let big_int = &big_integer.inner;
                    let zero_big_int: &BigInt = &Zero::zero();

                    let positive_term: Term = if big_int < zero_big_int {
                        let positive_big_int: BigInt = -1 * big_int;

                        positive_big_int.into_process(&process)
                    } else {
                        number
                    };

                    Some(positive_term)
                }
                Float => {
                    let float: &Float = number.unbox_reference();
                    let inner = float.inner;

                    match inner.partial_cmp(&0.0).unwrap() {
                        Ordering::Less => {
                            let positive_inner = inner.abs();
                            let positive_number: Term = positive_inner.into_process(&process);

                            Some(positive_number)
                        }
                        _ => Some(number),
                    }
                }
                _ => None,
            }
        }
        _ => None,
    };

    match option_abs {
        Some(abs) => Ok(abs),
        None => Err(badarg!()),
    }
}

/// `+/2` infix operator
pub fn add_2(augend: Term, addend: Term, process: &Process) -> Result {
    number_infix_operator!(augend, addend, process, checked_add, +)
}

/// `and/2` infix operator.
///
/// **NOTE: NOT SHORT-CIRCUITING!**  Use `andalso/2` for short-circuiting, but it doesn't enforce
/// that `right` is boolean.
pub fn and_2(left_boolean: Term, right_boolean: Term) -> Result {
    boolean_infix_operator!(left_boolean, right_boolean, &)
}

/// `andalso/2` infix operator.
///
/// Short-circuiting, but doesn't enforce `right` is boolean.  If you need to enforce `boolean` for
/// both operands, use `and_2`.
pub fn andalso_2(boolean: Term, term: Term) -> Result {
    let boolean_bool: bool = boolean.try_into()?;

    if boolean_bool {
        Ok(term)
    } else {
        // always `false.into()`, but this is faster
        Ok(boolean)
    }
}

pub fn append_element_2(tuple: Term, element: Term, process: &Process) -> Result {
    let internal: &Tuple = tuple.try_into_in_process(process)?;
    let new_tuple = internal.append_element(element, &process.heap.lock().unwrap());

    Ok(new_tuple.into())
}

/// `==/2` infix operator.  Unlike `=:=`, converts between floats and integers.
pub fn are_equal_after_conversion_2(left: Term, right: Term) -> Term {
    left.eq_after_conversion(&right).into()
}

/// `=:=/2` infix operator.  Unlike `==`, does not convert between floats and integers.
pub fn are_exactly_equal_2(left: Term, right: Term) -> Term {
    left.eq(&right).into()
}

/// `=/=/2` infix operator.  Unlike `!=`, does not convert between floats and integers.
pub fn are_exactly_not_equal_2(left: Term, right: Term) -> Term {
    left.ne(&right).into()
}

/// `/=/2` infix operator.  Unlike `=/=`, converts between floats and integers.
pub fn are_not_equal_after_conversion_2(left: Term, right: Term) -> Term {
    (!left.eq_after_conversion(&right)).into()
}

pub fn atom_to_binary_2(atom: Term, encoding: Term, process: &Process) -> Result {
    if atom.tag() == Atom {
        encoding.atom_to_encoding()?;
        let string = unsafe { atom.atom_to_string() };
        Ok(Term::slice_to_binary(string.as_bytes(), &process))
    } else {
        Err(badarg!())
    }
}

pub fn atom_to_list_1(atom: Term, process: &Process) -> Result {
    if atom.tag() == Atom {
        let string = unsafe { atom.atom_to_string() };
        Ok(Term::chars_to_list(string.chars(), &process))
    } else {
        Err(badarg!())
    }
}

// `band/2` infix operator.
pub fn band_2(left_integer: Term, right_integer: Term, process: &Process) -> Result {
    bitwise_infix_operator!(left_integer, right_integer, process, &)
}

pub fn binary_part_2(binary: Term, start_length: Term, process: &Process) -> Result {
    let option_result = match start_length.tag() {
        Boxed => {
            let unboxed: &Term = start_length.unbox_reference();

            match unboxed.tag() {
                Arity => {
                    let tuple: &Tuple = start_length.unbox_reference();

                    if tuple.len() == 2 {
                        Some(binary_part_3(binary, tuple[0], tuple[1], &process))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    };

    match option_result {
        Some(result) => result,
        None => Err(badarg!()),
    }
}

pub fn binary_part_3(binary: Term, start: Term, length: Term, process: &Process) -> Result {
    match binary.tag() {
        Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    heap_binary.part(start, length, &process)
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.part(start, length, &process)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn binary_to_atom_2(binary: Term, encoding: Term) -> Result {
    binary_existence_to_atom(binary, encoding, DoNotCare)
}

pub fn binary_to_existing_atom_2(binary: Term, encoding: Term) -> Result {
    binary_existence_to_atom(binary, encoding, Exists)
}

pub fn binary_to_float_1(binary: Term, process: &Process) -> Result {
    let string: String = binary.try_into()?;

    match string.parse::<f64>() {
        Ok(inner) => {
            match inner.classify() {
                FpCategory::Normal | FpCategory::Subnormal =>
                // unlike Rust, Erlang requires float strings to have a decimal point
                {
                    if (inner.fract() == 0.0) && !string.chars().any(|b| b == '.') {
                        Err(badarg!())
                    } else {
                        Ok(inner.into_process(&process))
                    }
                }
                // Erlang has no support for Nan, +inf or -inf
                FpCategory::Nan | FpCategory::Infinite => Err(badarg!()),
                FpCategory::Zero => {
                    // Erlang does not track the difference without +0 and -0.
                    Ok(inner.abs().into_process(&process))
                }
            }
        }
        Err(_) => Err(badarg!()),
    }
}

pub fn binary_to_integer_1(binary: Term, process: &Process) -> Result {
    let string: String = binary.try_into()?;
    let bytes = string.as_bytes();

    match BigInt::parse_bytes(bytes, 10) {
        Some(big_int) => {
            let term: Term = big_int.into_process(&process);

            Ok(term)
        }
        None => Err(badarg!()),
    }
}

pub fn binary_to_integer_2(binary: Term, base: Term, process: &Process) -> Result {
    let string: String = binary.try_into()?;
    let radix: usize = base.try_into()?;

    if 2 <= radix && radix <= 36 {
        let bytes = string.as_bytes();

        match BigInt::parse_bytes(bytes, radix as u32) {
            Some(big_int) => {
                let term: Term = big_int.into_process(&process);

                Ok(term)
            }
            None => Err(badarg!()),
        }
    } else {
        Err(badarg!())
    }
}

pub fn binary_to_list_1(binary: Term, process: &Process) -> Result {
    match binary.tag() {
        Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    Ok(heap_binary.to_list(&process))
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.to_list(&process)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

/// The one-based indexing for binaries used by this function is deprecated. New code is to use
/// [crate::otp::binary::bin_to_list] instead. All functions in module [crate::otp::binary]
/// consistently use zero-based indexing.
pub fn binary_to_list_3(binary: Term, start: Term, stop: Term, process: &Process) -> Result {
    let one_based_start_usize: usize = start.try_into()?;

    if 1 <= one_based_start_usize {
        let one_based_stop_usize: usize = stop.try_into()?;

        if one_based_start_usize <= one_based_stop_usize {
            let zero_based_start_usize = one_based_start_usize - 1;
            let zero_based_stop_usize = one_based_stop_usize - 1;

            let length_usize = zero_based_stop_usize - zero_based_start_usize + 1;

            otp::binary::bin_to_list(
                binary,
                zero_based_start_usize.into_process(&process),
                length_usize.into_process(&process),
                &process,
            )
        } else {
            Err(badarg!())
        }
    } else {
        Err(badarg!())
    }
}

pub fn binary_to_term_1(binary: Term, process: &Process) -> Result {
    binary_to_term_2(binary, Term::EMPTY_LIST, process)
}

pub fn binary_to_term_2(binary: Term, options: Term, process: &Process) -> Result {
    let to_term_options: ToTermOptions = options.try_into()?;

    match binary.tag() {
        Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    heap_binary.to_term(to_term_options, &process)
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.to_term(to_term_options, &process)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn bit_size_1(bit_string: Term, process: &Process) -> Result {
    match bit_string.tag() {
        Boxed => {
            let unboxed: &Term = bit_string.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = bit_string.unbox_reference();

                    Ok(heap_binary.bit_len())
                }
                Subbinary => {
                    let subbinary: &sub::Binary = bit_string.unbox_reference();

                    Ok(subbinary.bit_len())
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
    .map(|bit_size_usize| bit_size_usize.into_process(&process))
}

pub fn bitstring_to_list_1(bit_string: Term, process: &Process) -> Result {
    match bit_string.tag() {
        Boxed => {
            let unboxed: &Term = bit_string.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = bit_string.unbox_reference();

                    Ok(heap_binary.to_bitstring_list(&process))
                }
                Subbinary => {
                    let subbinary: &sub::Binary = bit_string.unbox_reference();

                    Ok(subbinary.to_bitstring_list(&process))
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

// `bnot/1` prefix operator.
pub fn bnot_1(integer: Term, process: &Process) -> Result {
    match integer.tag() {
        SmallInteger => {
            let integer_isize = unsafe { integer.small_integer_to_isize() };
            let output = !integer_isize;

            Ok(output.into_process(&process))
        }
        Boxed => {
            let unboxed: &Term = integer.unbox_reference();

            match unboxed.tag() {
                BigInteger => {
                    let big_integer: &big::Integer = integer.unbox_reference();
                    let big_int = &big_integer.inner;
                    let output_big_int = !big_int;

                    Ok(output_big_int.into_process(&process))
                }
                _ => Err(badarith!()),
            }
        }
        _ => Err(badarith!()),
    }
}

/// `bor/2` infix operator.
pub fn bor_2(left_integer: Term, right_integer: Term, process: &Process) -> Result {
    bitwise_infix_operator!(left_integer, right_integer, process, |)
}

pub const MAX_SHIFT: usize = std::mem::size_of::<isize>() * 8 - 1;

/// `bsl/2` infix operator.
pub fn bsl_2(integer: Term, shift: Term, process: &Process) -> Result {
    bitshift_infix_operator!(integer, shift, process, <<, >>)
}

/// `bsr/2` infix operator.
pub fn bsr_2(integer: Term, shift: Term, process: &Process) -> Result {
    bitshift_infix_operator!(integer, shift, process, >>, <<)
}

/// `bxor/2` infix operator.
pub fn bxor_2(left_integer: Term, right_integer: Term, process: &Process) -> Result {
    bitwise_infix_operator!(left_integer, right_integer, process, ^)
}

pub fn byte_size_1(bit_string: Term, process: &Process) -> Result {
    match bit_string.tag() {
        Boxed => {
            let unboxed: &Term = bit_string.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = bit_string.unbox_reference();

                    Ok(heap_binary.byte_len())
                }
                Subbinary => {
                    let subbinary: &sub::Binary = bit_string.unbox_reference();

                    Ok(subbinary.byte_len())
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
    .map(|byte_size_usize| byte_size_usize.into_process(&process))
}

pub fn cancel_timer_1(timer_reference: Term, process: &Process) -> Result {
    cancel_timer(timer_reference, Default::default(), process)
}

pub fn cancel_timer_2(timer_reference: Term, options: Term, process: &Process) -> Result {
    let cancel_timer_options: timer::cancel::Options = options.try_into()?;

    cancel_timer(timer_reference, cancel_timer_options, process)
}

pub fn ceil_1(number: Term, process: &Process) -> Result {
    match number.tag() {
        SmallInteger => Ok(number),
        Boxed => {
            let unboxed: &Term = number.unbox_reference();

            match unboxed.tag() {
                BigInteger => Ok(number),
                Float => {
                    let float: &Float = number.unbox_reference();
                    let inner = float.inner;
                    let ceil_inner = inner.ceil();

                    // skip creating a rug::Integer if float can fit in small integer.
                    let ceil_term = if (small::MIN as f64).max(float::INTEGRAL_MIN) <= ceil_inner
                        && ceil_inner <= (small::MAX as f64).min(float::INTEGRAL_MAX)
                    {
                        (ceil_inner as isize).into_process(&process)
                    } else {
                        let ceil_string = ceil_inner.to_string();
                        let ceil_bytes = ceil_string.as_bytes();
                        let big_int = BigInt::parse_bytes(ceil_bytes, 10).unwrap();

                        big_int.into_process(&process)
                    };

                    Ok(ceil_term)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

/// `++/2`
pub fn concatenate_2(list: Term, term: Term, process: &Process) -> Result {
    match list.tag() {
        EmptyList => Ok(term),
        List => {
            let cons: &Cons = unsafe { list.as_ref_cons_unchecked() };

            cons.concatenate(term, &process)
        }
        _ => Err(badarg!()),
    }
}

pub fn convert_time_unit_3(
    time: Term,
    from_unit: Term,
    to_unit: Term,
    process: &Process,
) -> Result {
    let time_big_int: BigInt = time.try_into()?;
    let from_unit_unit: crate::time::Unit = from_unit.try_into()?;
    let to_unit_unit: crate::time::Unit = to_unit.try_into()?;
    let converted =
        time::convert(time_big_int, from_unit_unit, to_unit_unit).into_process(&process);

    Ok(converted)
}

pub fn delete_element_2(index: Term, tuple: Term, process: &Process) -> Result {
    let initial_inner_tuple: &Tuple = tuple.try_into_in_process(&process)?;
    let index_zero_based: ZeroBasedIndex = index.try_into()?;

    initial_inner_tuple
        .delete_element(index_zero_based, &process.heap.lock().unwrap())
        .map(|final_inner_tuple| final_inner_tuple.into())
}

/// `div/2` infix operator.  Integer division.
pub fn div_2(dividend: Term, divisor: Term, process: &Process) -> Result {
    integer_infix_operator!(dividend, divisor, process, /)
}

/// `//2` infix operator.  Unlike `+/2`, `-/2` and `*/2` always promotes to `float` returns the
/// `float`.
pub fn divide_2(dividend: Term, divisor: Term, process: &Process) -> Result {
    let dividend_f64: f64 = dividend.try_into()?;
    let divisor_f64: f64 = divisor.try_into()?;

    if divisor_f64 == 0.0 {
        Err(badarith!())
    } else {
        let quotient_f64 = dividend_f64 / divisor_f64;

        Ok(quotient_f64.into_process(&process))
    }
}

pub fn element_2(index: Term, tuple: Term, process: &Process) -> Result {
    let inner_tuple: &Tuple = tuple.try_into_in_process(&process)?;
    let index_zero_based: ZeroBasedIndex = index.try_into()?;

    inner_tuple.element(index_zero_based)
}

/// `orelse/2` infix operator.
///
/// Short-circuiting, but doesn't enforce `right` is boolean.  If you need to enforce `boolean` for
/// both operands, use `or_2`.
pub fn orelse_2(boolean: Term, term: Term) -> Result {
    let boolean_bool: bool = boolean.try_into()?;

    if boolean_bool {
        // always `true.into()`, but this is faster
        Ok(boolean)
    } else {
        Ok(term)
    }
}

pub fn error_1(reason: Term) -> Result {
    Err(error!(reason))
}

pub fn error_2(reason: Term, arguments: Term) -> Result {
    Err(error!(reason, Some(arguments)))
}

pub fn exit_1(reason: Term) -> Result {
    Err(exit!(reason))
}

pub fn hd_1(list: Term) -> Result {
    let cons: &Cons = list.try_into()?;

    Ok(cons.head())
}

pub fn insert_element_3(index: Term, tuple: Term, element: Term, process: &Process) -> Result {
    let initial_inner_tuple: &Tuple = tuple.try_into_in_process(&process)?;
    let index_zero_based: ZeroBasedIndex = index.try_into()?;

    initial_inner_tuple
        .insert_element(index_zero_based, element, &process.heap.lock().unwrap())
        .map(|final_inner_tuple| final_inner_tuple.into())
}

/// Distribution is not supported at this time.  Always returns `false`.
pub fn is_alive_0() -> Term {
    false.into()
}

pub fn is_atom_1(term: Term) -> Term {
    term.is_atom().into()
}

pub fn is_binary_1(term: Term) -> Term {
    term.is_binary().into()
}

pub fn is_bitstring_1(term: Term) -> Term {
    term.is_bitstring().into()
}

pub fn is_boolean_1(term: Term) -> Term {
    term.is_boolean().into()
}

/// `=</2` infix operator.  Floats and integers are converted.
///
/// **NOTE: `=</2` is not a typo.  Unlike `>=/2`, which has the `=` second, Erlang put the `=` first
/// for `=</2`, instead of the more common `<=`.
pub fn is_equal_or_less_than_2(left: Term, right: Term) -> Term {
    left.le(&right).into()
}

pub fn is_float_1(term: Term) -> Term {
    term.is_float().into()
}

/// `>/2` infix operator.  Floats and integers are converted.
pub fn is_greater_than_2(left: Term, right: Term) -> Term {
    left.gt(&right).into()
}

/// `>=/2` infix operator.  Floats and integers are converted.
pub fn is_greater_than_or_equal_2(left: Term, right: Term) -> Term {
    left.ge(&right).into()
}

pub fn is_integer_1(term: Term) -> Term {
    term.is_integer().into()
}

/// `</2` infix operator.  Floats and integers are converted.
pub fn is_less_than_2(left: Term, right: Term) -> Term {
    left.lt(&right).into()
}

pub fn is_list_1(term: Term) -> Term {
    term.is_list().into()
}

pub fn is_map_1(term: Term) -> Term {
    term.is_map().into()
}

pub fn is_map_key_2(key: Term, map: Term, process: &Process) -> Result {
    let map_map: &Map = map.try_into_in_process(&process)?;

    Ok(map_map.is_key(key).into())
}

pub fn is_number_1(term: Term) -> Term {
    term.is_number().into()
}

pub fn is_pid_1(term: Term) -> Term {
    term.is_pid().into()
}

pub fn is_record_2(term: Term, record_tag: Term) -> Result {
    is_record(term, record_tag, None)
}

pub fn is_record_3(term: Term, record_tag: Term, size: Term) -> Result {
    is_record(term, record_tag, Some(size))
}

pub fn is_reference_1(term: Term) -> Term {
    match term.tag() {
        Boxed => {
            let unboxed: &Term = term.unbox_reference();

            match unboxed.tag() {
                LocalReference | ExternalReference => true,
                _ => false,
            }
        }
        _ => false,
    }
    .into()
}

pub fn is_tuple_1(term: Term) -> Term {
    term.is_tuple().into()
}

pub fn length_1(list: Term, process: &Process) -> Result {
    match list.count() {
        Some(length) => Ok(length.into_process(process)),
        None => Err(badarg!()),
    }
}

pub fn list_to_atom_1(string: Term) -> Result {
    list_to_atom(string, DoNotCare)
}

pub fn list_to_existing_atom_1(string: Term) -> Result {
    list_to_atom(string, Exists)
}

pub fn list_to_binary_1(iolist: Term, process: &Process) -> Result {
    match iolist.tag() {
        EmptyList | List => {
            let mut byte_vec: Vec<u8> = Vec::new();
            let mut stack: Vec<Term> = vec![iolist];

            while let Some(top) = stack.pop() {
                match top.tag() {
                    SmallInteger => {
                        let top_isize = unsafe { top.small_integer_to_isize() };
                        let top_byte = top_isize.try_into().map_err(|_| badarg!())?;

                        byte_vec.push(top_byte);
                    }
                    EmptyList => (),
                    List => {
                        let cons: &Cons = unsafe { top.as_ref_cons_unchecked() };

                        // @type iolist :: maybe_improper_list(byte() | binary() | iolist(),
                        // binary() | []) means that `byte()` isn't allowed
                        // for `tail`s unlike `head`.

                        let tail = cons.tail();

                        if tail.tag() == SmallInteger {
                            return Err(badarg!());
                        } else {
                            stack.push(tail);
                        }

                        stack.push(cons.head());
                    }
                    Boxed => {
                        let unboxed: &Term = top.unbox_reference();

                        match unboxed.tag() {
                            HeapBinary => {
                                let heap_binary: &heap::Binary = top.unbox_reference();

                                byte_vec.extend_from_slice(heap_binary.as_slice());
                            }
                            Subbinary => {
                                let subbinary: &sub::Binary = top.unbox_reference();

                                if subbinary.bit_count == 0 {
                                    byte_vec.extend(subbinary.byte_iter());
                                } else {
                                    return Err(badarg!());
                                }
                            }
                            _ => return Err(badarg!()),
                        }
                    }
                    _ => return Err(badarg!()),
                }
            }

            Ok(Term::slice_to_binary(byte_vec.as_slice(), &process))
        }
        _ => Err(badarg!()),
    }
}

pub fn list_to_bitstring_1(iolist: Term, process: &Process) -> Result {
    match iolist.tag() {
        EmptyList | List => {
            let mut byte_vec: Vec<u8> = Vec::new();
            let mut bit_offset = 0;
            let mut tail_byte = 0;
            let mut stack: Vec<Term> = vec![iolist];

            while let Some(top) = stack.pop() {
                match top.tag() {
                    SmallInteger => {
                        let top_isize = unsafe { top.small_integer_to_isize() };
                        let top_byte = top_isize.try_into().map_err(|_| badarg!())?;

                        if bit_offset == 0 {
                            byte_vec.push(top_byte);
                        } else {
                            tail_byte |= top_byte >> bit_offset;
                            byte_vec.push(tail_byte);

                            tail_byte = top_byte << (8 - bit_offset);
                        }
                    }
                    EmptyList => (),
                    List => {
                        let cons: &Cons = unsafe { top.as_ref_cons_unchecked() };

                        // @type bitstring_list ::
                        //   maybe_improper_list(byte() | bitstring() | bitstring_list(),
                        //                       bitstring() | [])
                        // means that `byte()` isn't allowed for `tail`s unlike `head`.

                        let tail = cons.tail();

                        if tail.tag() == SmallInteger {
                            return Err(badarg!());
                        } else {
                            stack.push(tail);
                        }

                        stack.push(cons.head());
                    }
                    Boxed => {
                        let unboxed: &Term = top.unbox_reference();

                        match unboxed.tag() {
                            HeapBinary => {
                                let heap_binary: &heap::Binary = top.unbox_reference();

                                if bit_offset == 0 {
                                    byte_vec.extend_from_slice(heap_binary.as_slice());
                                } else {
                                    for byte in heap_binary.byte_iter() {
                                        tail_byte |= byte >> bit_offset;
                                        byte_vec.push(tail_byte);

                                        tail_byte = byte << (8 - bit_offset);
                                    }
                                }
                            }
                            Subbinary => {
                                let subbinary: &sub::Binary = top.unbox_reference();

                                if bit_offset == 0 {
                                    byte_vec.extend(subbinary.byte_iter());
                                } else {
                                    for byte in subbinary.byte_iter() {
                                        tail_byte |= byte >> bit_offset;
                                        byte_vec.push(tail_byte);

                                        tail_byte = byte << (8 - bit_offset);
                                    }
                                }

                                if 0 < subbinary.bit_count {
                                    for bit in subbinary.bit_count_iter() {
                                        tail_byte |= bit << (7 - bit_offset);

                                        if bit_offset == 7 {
                                            byte_vec.push(tail_byte);
                                            bit_offset = 0;
                                            tail_byte = 0;
                                        } else {
                                            bit_offset += 1;
                                        }
                                    }
                                }
                            }
                            _ => return Err(badarg!()),
                        }
                    }
                    _ => return Err(badarg!()),
                }
            }

            if bit_offset == 0 {
                Ok(Term::slice_to_binary(byte_vec.as_slice(), &process))
            } else {
                let byte_count = byte_vec.len();
                byte_vec.push(tail_byte);
                let original = Term::slice_to_binary(byte_vec.as_slice(), &process);

                Ok(Term::subbinary(
                    original, 0, 0, byte_count, bit_offset, &process,
                ))
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn list_to_pid_1(string: Term, process: &Process) -> Result {
    let cons: &Cons = string.try_into()?;

    cons.to_pid(&process)
}

pub fn list_to_tuple_1(list: Term, process: &Process) -> Result {
    match list.tag() {
        EmptyList => Ok(Term::slice_to_tuple(&[], &process)),
        List => {
            let cons: &Cons = unsafe { list.as_ref_cons_unchecked() };

            cons.to_tuple(&process)
        }
        _ => Err(badarg!()),
    }
}

pub fn make_ref_0(process: &Process) -> Term {
    Term::next_local_reference(&process)
}

pub fn map_get_2(key: Term, map: Term, process: &Process) -> Result {
    let map_map: &Map = map.try_into_in_process(&process)?;

    match map_map.get(key) {
        Some(value) => Ok(value),
        None => Err(badkey!(key, &process)),
    }
}

pub fn map_size_1(map: Term, process: &Process) -> Result {
    let map_map: &Map = map.try_into_in_process(&process)?;

    Ok(map_map.size().into_process(&process))
}

/// `max/2`
///
/// Returns the largest of `Term1` and `Term2`. If the terms are equal, `Term1` is returned.
pub fn max_2(term1: Term, term2: Term) -> Term {
    // Flip the order because for Rust `max` returns the second argument when equal, but Erlang
    // returns the first
    term2.max(term1)
}

/// `min/2`
///
/// Returns the smallest of `Term1` and `Term2`. If the terms are equal, `Term1` is returned.
pub fn min_2(term1: Term, term2: Term) -> Term {
    term1.min(term2)
}

pub fn monotonic_time_0(process: &Process) -> Term {
    monotonic::time(Native).into_process(process)
}

pub fn monotonic_time_1(unit: Term, process: &Process) -> Result {
    let unit_unit: crate::time::Unit = unit.try_into()?;

    Ok(monotonic::time(unit_unit).into_process(process))
}

/// `*/2` infix operator
pub fn multiply_2(multiplier: Term, multiplicand: Term, process: &Process) -> Result {
    number_infix_operator!(multiplier, multiplicand, process, checked_mul, *)
}

/// `-/1` prefix operator.
pub fn negate_1(number: Term, process: &Process) -> Result {
    match number.tag() {
        SmallInteger => {
            let number_isize = unsafe { number.small_integer_to_isize() };
            let negated_isize = -number_isize;

            Ok(negated_isize.into_process(&process))
        }
        Boxed => {
            let unboxed: &Term = number.unbox_reference();

            match unboxed.tag() {
                BigInteger => {
                    let big_integer: &big::Integer = number.unbox_reference();
                    let negated_big_int = -&big_integer.inner;

                    Ok(negated_big_int.into_process(&process))
                }
                Float => {
                    let float: &Float = number.unbox_reference();
                    let negated_f64 = -float.inner;

                    Ok(negated_f64.into_process(&process))
                }
                _ => Err(badarith!()),
            }
        }
        _ => Err(badarith!()),
    }
}

pub fn node_0() -> Term {
    Term::str_to_atom(node::DEAD, DoNotCare).unwrap()
}

/// `not/1` prefix operator.
pub fn not_1(boolean: Term) -> Result {
    let boolean_bool: bool = boolean.try_into()?;
    let output = !boolean_bool;

    Ok(output.into())
}

/// `+/1` prefix operator.
pub fn number_or_badarith_1(term: Term) -> Result {
    if term.is_number() {
        Ok(term)
    } else {
        Err(badarith!())
    }
}

/// `or/2` infix operator.
///
/// **NOTE: NOT SHORT-CIRCUITING!**
pub fn or_2(left_boolean: Term, right_boolean: Term) -> Result {
    boolean_infix_operator!(left_boolean, right_boolean, |)
}

pub fn raise_3(class: Term, reason: Term, stacktrace: Term) -> Result {
    let class_class: Class = class.try_into()?;

    if stacktrace::is(stacktrace) {
        Err(raise!(class_class, reason, Some(stacktrace)))
    } else {
        Err(badarg!())
    }
}

pub fn read_timer_1(timer_reference: Term, process: &Process) -> Result {
    read_timer(timer_reference, Default::default(), process)
}

pub fn read_timer_2(timer_reference: Term, options: Term, process: &Process) -> Result {
    let read_timer_options: timer::read::Options = options.try_into()?;

    read_timer(timer_reference, read_timer_options, process)
}

pub fn register_2(name: Term, pid_or_port: Term, process_arc: Arc<Process>) -> Result {
    match name.tag() {
        Atom => match unsafe { name.atom_to_string() }.as_ref().as_ref() {
            "undefined" => Err(badarg!()),
            _ => {
                let writable_registry = registry::RW_LOCK_REGISTERED_BY_NAME.write().unwrap();

                if !writable_registry.contains_key(&name) {
                    match pid_or_port.tag() {
                        LocalPid => match pid_to_self_or_process(pid_or_port, &process_arc) {
                            Some(pid_process_arc) => {
                                pid_process_arc.register_in(writable_registry, name)
                            }
                            None => Err(badarg!()),
                        },
                        _ => Err(badarg!()),
                    }
                } else {
                    Err(badarg!())
                }
            }
        },
        _ => Err(badarg!()),
    }
}

pub fn registered_0(process: &Process) -> Term {
    registry::RW_LOCK_REGISTERED_BY_NAME
        .read()
        .unwrap()
        .keys()
        .fold(Term::EMPTY_LIST, |acc, name| {
            Term::cons(name.clone(), acc, process)
        })
}

/// `rem/2` infix operator.  Integer remainder.
pub fn rem_2(dividend: Term, divisor: Term, process: &Process) -> Result {
    integer_infix_operator!(dividend, divisor, process, %)
}

pub fn self_0(process: &Process) -> Term {
    process.pid
}

pub fn send_2(destination: Term, message: Term, process: &Process) -> Result {
    send(destination, message, Default::default(), process).map(|sent| match sent {
        Sent::Sent => message,
        _ => unreachable!(),
    })
}

// `send(destination, message, [nosuspend])` is used in `gen.erl`, which is used by `gen_server.erl`
// See https://github.com/erlang/otp/blob/8f6d45ddc8b2b12376c252a30b267a822cad171a/lib/stdlib/src/gen.erl#L167
pub fn send_3(destination: Term, message: Term, options: Term, process: &Process) -> Result {
    let send_options: send::Options = options.try_into()?;

    send(destination, message, send_options, process).map(|sent| match sent {
        Sent::Sent => Term::str_to_atom("ok", DoNotCare).unwrap(),
        Sent::ConnectRequired => Term::str_to_atom("noconnect", DoNotCare).unwrap(),
        Sent::SuspendRequired => Term::str_to_atom("nosuspend", DoNotCare).unwrap(),
    })
}

pub fn send_after_3(
    time: Term,
    destination: Term,
    message: Term,
    process_arc: Arc<Process>,
) -> Result {
    start_timer(
        time,
        destination,
        Timeout::Message,
        message,
        Default::default(),
        process_arc,
    )
}

pub fn send_after_4(
    time: Term,
    destination: Term,
    message: Term,
    options: Term,
    process_arc: Arc<Process>,
) -> Result {
    let timer_start_options: timer::start::Options = options.try_into()?;

    start_timer(
        time,
        destination,
        Timeout::Message,
        message,
        timer_start_options,
        process_arc,
    )
}

pub fn setelement_3(index: Term, tuple: Term, value: Term, process: &Process) -> Result {
    let inner_tuple: &Tuple = tuple.try_into_in_process(&process)?;
    let index_zero_based: ZeroBasedIndex = index.try_into()?;

    inner_tuple
        .setelement(index_zero_based, value, &process.heap.lock().unwrap())
        .map(|new_inner_tuple| new_inner_tuple.into())
}

pub fn size_1(binary_or_tuple: Term, process: &Process) -> Result {
    match binary_or_tuple.tag() {
        Boxed => {
            let unboxed: &Term = binary_or_tuple.unbox_reference();

            match unboxed.tag() {
                Arity => {
                    let tuple: &Tuple = binary_or_tuple.unbox_reference();

                    Ok(tuple.size())
                }
                HeapBinary => {
                    let heap_binary: &heap::Binary = binary_or_tuple.unbox_reference();

                    Ok(heap_binary.size())
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary_or_tuple.unbox_reference();

                    Ok(subbinary.size())
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
    .map(|integer| integer.into_process(&process))
}

pub fn spawn_3(module: Term, function: Term, arguments: Term, process: &Process) -> Result {
    let option_pid = if (module.tag() == Atom) && (function.tag() == Atom) {
        match arguments.tag() {
            EmptyList => {
                let arc_process =
                    Scheduler::spawn(process, module, function, arguments, code::apply_fn());

                Some(arc_process.pid)
            }
            List => {
                let cons: &Cons = unsafe { arguments.as_ref_cons_unchecked() };

                if cons.is_proper() {
                    let arc_process =
                        Scheduler::spawn(process, module, function, arguments, code::apply_fn());

                    Some(arc_process.pid)
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };

    match option_pid {
        Some(pid) => Ok(pid),
        None => Err(badarg!()),
    }
}

pub fn split_binary_2(binary: Term, position: Term, process: &Process) -> Result {
    let index: usize = position.try_into()?;

    match binary.tag() {
        Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    if index == 0 {
                        let empty_prefix = Term::subbinary(binary, index, 0, 0, 0, &process);

                        // Don't make a subbinary of the suffix since it is the same as the
                        // `binary`.
                        Ok(Term::slice_to_tuple(&[empty_prefix, binary], &process))
                    } else {
                        let heap_binary: &heap::Binary = binary.unbox_reference();
                        let byte_length = heap_binary.byte_len();

                        if index < byte_length {
                            let prefix = Term::subbinary(binary, 0, 0, index, 0, &process);
                            let suffix =
                                Term::subbinary(binary, index, 0, byte_length - index, 0, &process);

                            Ok(Term::slice_to_tuple(&[prefix, suffix], &process))
                        } else if index == byte_length {
                            let empty_suffix = Term::subbinary(binary, index, 0, 0, 0, &process);

                            // Don't make a subbinary of the prefix since it is the same as the
                            // `binary`.
                            Ok(Term::slice_to_tuple(&[binary, empty_suffix], &process))
                        } else {
                            Err(badarg!())
                        }
                    }
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    if index == 0 {
                        let empty_prefix = Term::subbinary(
                            subbinary.original,
                            subbinary.byte_offset + index,
                            subbinary.bit_offset,
                            0,
                            0,
                            &process,
                        );

                        // Don't make a subbinary of the suffix since it is the same as the
                        // `binary`.
                        Ok(Term::slice_to_tuple(&[empty_prefix, binary], &process))
                    } else {
                        // byte_length includes +1 byte if bits
                        let byte_length = subbinary.byte_len();

                        if index < byte_length {
                            let original = subbinary.original;
                            let byte_offset = subbinary.byte_offset;
                            let bit_offset = subbinary.bit_offset;
                            let prefix = Term::subbinary(
                                original,
                                byte_offset,
                                bit_offset,
                                index,
                                0,
                                &process,
                            );
                            let suffix = Term::subbinary(
                                original,
                                byte_offset + index,
                                bit_offset,
                                // byte_count does not include bits
                                subbinary.byte_count - index,
                                subbinary.bit_count,
                                &process,
                            );

                            Ok(Term::slice_to_tuple(&[prefix, suffix], &process))
                        } else if (index == byte_length) && (subbinary.bit_count == 0) {
                            let empty_suffix = Term::subbinary(
                                subbinary.original,
                                subbinary.byte_offset + index,
                                subbinary.bit_offset,
                                0,
                                0,
                                &process,
                            );

                            Ok(Term::slice_to_tuple(&[binary, empty_suffix], &process))
                        } else {
                            Err(badarg!())
                        }
                    }
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn start_timer_3(
    time: Term,
    destination: Term,
    message: Term,
    process_arc: Arc<Process>,
) -> Result {
    start_timer(
        time,
        destination,
        Timeout::TimeoutTuple,
        message,
        Default::default(),
        process_arc,
    )
}

pub fn start_timer_4(
    time: Term,
    destination: Term,
    message: Term,
    options: Term,
    process_arc: Arc<Process>,
) -> Result {
    let timer_start_options: timer::start::Options = options.try_into()?;

    start_timer(
        time,
        destination,
        Timeout::TimeoutTuple,
        message,
        timer_start_options,
        process_arc,
    )
}

/// `-/2` infix operator
pub fn subtract_2(minuend: Term, subtrahend: Term, process: &Process) -> Result {
    number_infix_operator!(minuend, subtrahend, process, checked_sub, -)
}

pub fn subtract_list_2(minuend: Term, subtrahend: Term, process: &Process) -> Result {
    match (minuend.tag(), subtrahend.tag()) {
        (EmptyList, EmptyList) => Ok(minuend),
        (EmptyList, List) => {
            let subtrahend_cons: &Cons = unsafe { subtrahend.as_ref_cons_unchecked() };

            if subtrahend_cons.is_proper() {
                Ok(minuend)
            } else {
                Err(badarg!())
            }
        }
        (List, EmptyList) => {
            if unsafe { minuend.as_ref_cons_unchecked() }.is_proper() {
                Ok(minuend)
            } else {
                Err(badarg!())
            }
        }
        (List, List) => {
            let minuend_cons: &Cons = unsafe { minuend.as_ref_cons_unchecked() };
            let subtrahend_cons: &Cons = unsafe { subtrahend.as_ref_cons_unchecked() };

            minuend_cons.subtract(subtrahend_cons, &process)
        }
        _ => Err(badarg!()),
    }
}

pub fn throw_1(reason: Term) -> Result {
    Err(throw!(reason))
}

pub fn tl_1(list: Term) -> Result {
    let cons: &Cons = list.try_into()?;

    Ok(cons.tail())
}

pub fn tuple_size_1(tuple: Term, process: &Process) -> Result {
    match tuple.tag() {
        Boxed => {
            let unboxed: &Term = tuple.unbox_reference();

            match unboxed.tag() {
                Arity => {
                    let tuple: &Tuple = tuple.unbox_reference();

                    Ok(tuple.size().into_process(&process))
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn tuple_to_list_1(tuple: Term, process: &Process) -> Result {
    match tuple.tag() {
        Boxed => {
            let unboxed: &Term = tuple.unbox_reference();

            match unboxed.tag() {
                Arity => {
                    let tuple: &Tuple = tuple.unbox_reference();

                    Ok(tuple.to_list(&process))
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn unregister_1(name: Term) -> Result {
    match name.tag() {
        Atom => {
            let mut writable_registry = registry::RW_LOCK_REGISTERED_BY_NAME.write().unwrap();

            match writable_registry.remove(&name) {
                Some(Registered::Process(weak_process)) => match weak_process.upgrade() {
                    Some(arc_process) => {
                        let mut writable_registerd_name =
                            arc_process.registered_name.write().unwrap();
                        *writable_registerd_name = None;

                        Ok(true.into())
                    }
                    None => Err(badarg!()),
                },
                None => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

pub fn whereis_1(name: Term) -> Result {
    match name.tag() {
        Atom => {
            let readable_registry = registry::RW_LOCK_REGISTERED_BY_NAME.read().unwrap();

            match readable_registry.get(&name) {
                Some(Registered::Process(weak_process)) => match weak_process.upgrade() {
                    Some(arc_process) => Ok(arc_process.pid),
                    None => Ok(Term::str_to_atom("undefined", DoNotCare).unwrap()),
                },
                None => Ok(Term::str_to_atom("undefined", DoNotCare).unwrap()),
            }
        }
        _ => Err(badarg!()),
    }
}

/// `xor/2` infix operator.
///
/// **NOTE: NOT SHORT-CIRCUITING!**
pub fn xor_2(left_boolean: Term, right_boolean: Term) -> Result {
    boolean_infix_operator!(left_boolean, right_boolean, ^)
}

// Private

fn binary_existence_to_atom(binary: Term, encoding: Term, existence: Existence) -> Result {
    encoding.atom_to_encoding()?;

    match binary.tag() {
        Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    heap_binary
                        .to_atom_index(existence)
                        .ok_or_else(|| badarg!())
                }
                Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.to_atom_index(existence)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
    .map(|atom_index| atom_index.into())
}

fn cancel_timer(
    timer_reference: Term,
    options: timer::cancel::Options,
    process: &Process,
) -> Result {
    match timer_reference.tag() {
        Boxed => {
            let unboxed_timer_reference: &Term = timer_reference.unbox_reference();

            match unboxed_timer_reference.tag() {
                LocalReference => {
                    let local_reference: &reference::local::Reference =
                        timer_reference.unbox_reference();

                    let canceled = timer::cancel(local_reference);

                    let term = if options.info {
                        let canceled_term = match canceled {
                            Some(milliseconds_remaining) => {
                                milliseconds_remaining.into_process(process)
                            }
                            None => false.into(),
                        };

                        if options.r#async {
                            let cancel_timer_message = Term::slice_to_tuple(
                                &[
                                    Term::str_to_atom("cancel_timer", DoNotCare).unwrap(),
                                    timer_reference,
                                    canceled_term,
                                ],
                                process,
                            );
                            process.send_from_self(cancel_timer_message);

                            Term::str_to_atom("ok", DoNotCare).unwrap()
                        } else {
                            canceled_term
                        }
                    } else {
                        Term::str_to_atom("ok", DoNotCare).unwrap()
                    };

                    Ok(term)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

fn is_record(term: Term, record_tag: Term, size: Option<Term>) -> Result {
    match term.tag() {
        Boxed => {
            let unboxed: &Term = term.unbox_reference();

            match unboxed.tag() {
                Arity => {
                    let tuple: &Tuple = term.unbox_reference();

                    tuple.is_record(record_tag, size)
                }
                _ => Ok(false.into()),
            }
        }
        _ => Ok(false.into()),
    }
}

fn list_to_atom(string: Term, existence: Existence) -> Result {
    match string.tag() {
        EmptyList => Term::str_to_atom("", existence).ok_or_else(|| badarg!()),
        List => {
            let cons: &Cons = unsafe { string.as_ref_cons_unchecked() };

            cons.to_atom(existence)
        }
        _ => Err(badarg!()),
    }
}

fn read_timer(timer_reference: Term, options: timer::read::Options, process: &Process) -> Result {
    match timer_reference.tag() {
        Boxed => {
            let unboxed_timer_reference: &Term = timer_reference.unbox_reference();

            match unboxed_timer_reference.tag() {
                LocalReference => {
                    let local_reference: &reference::local::Reference =
                        timer_reference.unbox_reference();

                    let read = timer::read(local_reference);

                    let read_term = match read {
                        Some(milliseconds_remaining) => {
                            milliseconds_remaining.into_process(process)
                        }
                        None => false.into(),
                    };

                    let term = if options.r#async {
                        let read_timer_message = Term::slice_to_tuple(
                            &[
                                Term::str_to_atom("read_timer", DoNotCare).unwrap(),
                                timer_reference,
                                read_term,
                            ],
                            process,
                        );
                        process.send_from_self(read_timer_message);

                        Term::str_to_atom("ok", DoNotCare).unwrap()
                    } else {
                        read_term
                    };

                    Ok(term)
                }
                _ => Err(badarg!()),
            }
        }
        _ => Err(badarg!()),
    }
}

fn start_timer(
    time: Term,
    destination: Term,
    timeout: Timeout,
    message: Term,
    options: timer::start::Options,
    process_arc: Arc<Process>,
) -> Result {
    if time.is_integer() {
        let reference_frame_milliseconds: Milliseconds = time.try_into()?;

        let absolute_milliseconds = match options.reference_frame {
            ReferenceFrame::Relative => {
                monotonic::time_in_milliseconds() + reference_frame_milliseconds
            }
            ReferenceFrame::Absolute => reference_frame_milliseconds,
        };

        match destination.tag() {
            // Registered names are looked up at time of send
            Atom => Ok(timer::start(
                absolute_milliseconds,
                timer::Destination::Name(destination),
                timeout,
                message,
                &process_arc,
            )),
            // PIDs are looked up at time of create.  If they don't exist, they still return a
            // LocalReference.
            LocalPid => match pid_to_self_or_process(destination, &process_arc) {
                Some(pid_process_arc) => Ok(timer::start(
                    absolute_milliseconds,
                    timer::Destination::Process(Arc::downgrade(&pid_process_arc)),
                    timeout,
                    message,
                    &process_arc,
                )),
                None => Ok(Term::next_local_reference(&process_arc)),
            },
            _ => Err(badarg!()),
        }
    } else {
        Err(badarg!())
    }
}