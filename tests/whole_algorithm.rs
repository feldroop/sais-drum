use std::iter;

use proptest::prelude::*;
use rand::seq::SliceRandom;

use sais_drum::{Character, IndexStorage, SaisBuilder};

// example from
// https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
static ABC_TEXT: &[u8] = b"ababcabcabba";

#[test]
fn u8_abc_text() {
    let result = SaisBuilder::<_>::new().construct_suffix_array(ABC_TEXT);
    let expected_suffix_array = [11, 0, 8, 5, 2, 10, 1, 9, 6, 3, 7, 4];

    assert!(is_suffix_array(&result, ABC_TEXT));
    assert_eq!(result, expected_suffix_array);
}

#[test]
fn len_0_1_2_texts() {
    let empty_text: [u8; 0] = [];
    let result_zero = SaisBuilder::<_>::new().construct_suffix_array(&empty_text);
    let result_one = SaisBuilder::<_>::new().construct_suffix_array(&[42u8]);
    let result_two = SaisBuilder::<_>::new()
        .with_max_char(42usize)
        .construct_suffix_array(&[42usize, 3]);

    assert_eq!(result_zero, []);
    assert_eq!(result_one, [0]);
    assert_eq!(result_two, [1, 0]);
}

#[test]
fn no_lms_mini_text() {
    let text = [0u8, 1];
    let suffix_array = SaisBuilder::<_>::new().construct_suffix_array(&text);

    assert_eq!(suffix_array, [0, 1]);
}

#[test]
fn one_lms_mini_text() {
    let text = b"424";
    let suffix_array = SaisBuilder::<_>::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [1, 2, 0]);
}

#[test]
fn two_lms_mini_text() {
    let text = b"yxyxy";
    let suffix_array = SaisBuilder::<_>::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [3, 1, 4, 2, 0]);
}

#[test]
fn single_char_text() {
    let text = vec![0u8; 10_000];
    let suffix_array = SaisBuilder::<_>::new()
        .with_max_char(0)
        .construct_suffix_array(&text);

    let expected_suffix_array: Vec<_> = (0..10_000).rev().collect();

    assert_eq!(suffix_array, expected_suffix_array);
}

#[test]
fn detrimental_text() {
    // this text will contain many short, but different LMS-substrings
    // (bad for memory usage of algorithm and might therefore trigger some edge cases)
    // they are not all different though, as that would trigger the recursion base case
    let mut base_chars = Vec::new();

    for character in 0..2000u16 {
        base_chars.extend(iter::repeat_n(character * 2, 10));
    }

    base_chars.shuffle(&mut rand::rng());

    let mut text = Vec::new();

    for base_char in base_chars {
        // add small subsequence to text that will always contain a short LMS substring
        text.extend_from_slice(&[
            base_char + 1,
            base_char,
            base_char + 1,
            base_char,
            base_char + 1,
        ]);
    }

    let maybe_suffix_array = SaisBuilder::<_>::new().construct_suffix_array(&text);
    assert!(is_suffix_array(&maybe_suffix_array, &text));
}

#[test]
fn failing_proptest_example1() {
    // this example tests a rarely used code path in the buffer layout that lead to a bug at some point
    // is this code path, only the persistent bucket buffer is stored inside the main buffer
    let text = [
        0u8, 0, 98, 0, 0, 128, 0, 0, 0, 58, 0, 127, 0, 0, 42, 0, 0, 89, 0, 0, 0, 0, 28, 0, 0, 0, 0,
        74, 0, 0, 10, 0, 41, 0, 5, 0, 68, 0, 171, 0, 37, 0, 45, 0, 137, 0, 28, 0, 77, 0, 80, 0, 0,
        0, 0, 0, 0, 18, 0, 0, 10, 0, 0, 16, 0, 72, 0, 0, 0, 15, 0, 0, 0, 0, 34, 0, 0, 0, 0, 0, 38,
        0, 0, 40, 0, 0, 0, 112, 0, 0, 0, 96, 0, 0, 0, 0, 117, 0, 0, 59, 0, 0, 43, 0, 18, 0, 78, 0,
        120, 0, 64, 0, 13, 0, 16, 0, 182, 0, 0, 0, 5, 0, 0, 18, 0, 0, 55, 0, 0, 95, 0, 60, 0, 90,
        0, 55, 0, 7, 0, 55, 0, 16, 1, 77, 0, 111, 0, 7, 1, 70, 0, 0, 51, 0, 0, 0, 45, 0, 0, 0, 0,
        25, 0, 0, 5, 0, 0, 0, 4, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 198, 0, 0, 69, 0, 97, 0, 0, 96, 0,
        0, 0, 81, 0, 0, 93, 0, 0, 0, 58, 1, 0, 0, 2, 0, 0, 6, 0, 0, 0, 112, 1, 36, 0, 0, 40, 1, 0,
        53, 0, 0, 0, 0, 71, 0, 15, 0, 0, 30, 0, 75, 0, 23, 0, 68, 1, 0, 174, 0, 49, 0, 37, 1, 125,
        0, 202, 0, 28, 0, 0, 0, 0, 0, 119, 0, 0, 0, 50, 0, 14, 0, 0, 0, 67, 0, 0, 94, 0, 13, 1, 0,
        2, 0, 102, 0, 0, 0, 109, 0, 75, 1, 0, 168, 0, 111, 1, 27, 0, 0, 0, 0, 0, 29, 0, 40, 0, 0,
        0, 66, 0, 57, 0, 43, 0, 0, 53, 0, 127, 1, 0, 125, 0, 202, 1, 0, 99, 0, 100, 0, 0, 0, 78, 0,
        36, 0, 0, 0, 15, 1, 0, 80, 1, 0, 0, 59, 1, 0, 30, 0, 0, 0, 143, 0, 0, 50, 0, 0, 0, 0, 0,
        15, 0, 42, 0, 13, 1, 1, 0, 27, 0, 0, 34, 0, 0, 39, 0, 47, 0, 0, 71, 0, 76, 0, 0, 95, 1, 0,
        0, 4, 0, 0, 114, 0, 1,
    ];

    let maybe_suffix_array = SaisBuilder::<_>::new().construct_suffix_array(&text);
    assert!(is_suffix_array(&maybe_suffix_array, &text));
}

fn construct_and_test_suffix_array<C: Character, I: IndexStorage>(text: &[C]) {
    let suffix_array = SaisBuilder::<C, I>::new().construct_suffix_array(&text);

    assert!(is_suffix_array(&suffix_array, &text));
}

fn is_suffix_array<C: Character, I: IndexStorage>(maybe_suffix_array: &[I], text: &[C]) -> bool {
    if maybe_suffix_array.len() != text.len() {
        return false;
    }

    for suffix_indices in maybe_suffix_array.windows(2) {
        if text[suffix_indices[0].as_()..] > text[suffix_indices[1].as_()..] {
            return false;
        }
    }

    true
}

proptest! {
    // default is 256 and I'd like some more test cases that need to pass
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn correctness_random_texts(text in prop::collection::vec(any::<u8>(), 0..1000), type_index in 0..4) {
        match type_index {
            0 => construct_and_test_suffix_array::<u8, u16>(&text),
            1 => construct_and_test_suffix_array::<u8, u32>(&text),
            2 => construct_and_test_suffix_array::<u8, u64>(&text),
            3 => construct_and_test_suffix_array::<u8, usize>(&text),
            _ => unreachable!()
        }
    }
}
