use std::iter;

use lipsum::lipsum_words_with_rng;
use proptest::prelude::*;
use rand::seq::SliceRandom;

use sais_drum::{Character, SaisBuilder};

// example from
// https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
static ABC_TEXT: &[u8] = b"ababcabcabba";

#[test]
fn u8_abc_text() {
    let result = SaisBuilder::new().construct_suffix_array(ABC_TEXT);
    let expected_suffix_array = [11, 0, 8, 5, 2, 10, 1, 9, 6, 3, 7, 4];

    assert!(is_suffix_array(&result, ABC_TEXT));
    assert_eq!(result, expected_suffix_array);
}

#[test]
fn len_0_1_2_texts() {
    let empty_text: [u8; 0] = [];
    let result_zero = SaisBuilder::new().construct_suffix_array(&empty_text);
    let result_one = SaisBuilder::new().construct_suffix_array(&[42u8]);
    let result_two = SaisBuilder::new()
        .with_max_char(42usize)
        .construct_suffix_array(&[42usize, 3]);

    assert_eq!(result_zero, []);
    assert_eq!(result_one, [0]);
    assert_eq!(result_two, [1, 0]);
}

#[test]
fn no_lms_mini_text() {
    let text = [0u8, 1];
    let suffix_array = SaisBuilder::new().construct_suffix_array(&text);

    assert_eq!(suffix_array, [0, 1]);
}

#[test]
fn one_lms_mini_text() {
    let text = b"424";
    let suffix_array = SaisBuilder::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [1, 2, 0]);
}

#[test]
fn two_lms_mini_text() {
    let text = b"yxyxy";
    let suffix_array = SaisBuilder::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [3, 1, 4, 2, 0]);
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

    base_chars.shuffle(&mut rand::thread_rng());

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

    let maybe_suffix_array = SaisBuilder::new().construct_suffix_array(&text);
    assert!(is_suffix_array(&maybe_suffix_array, &text))
}

fn is_suffix_array<C: Character>(maybe_suffix_array: &[usize], text: &[C]) -> bool {
    if maybe_suffix_array.len() != text.len() {
        return false;
    }

    for suffix_indices in maybe_suffix_array.windows(2) {
        if text[suffix_indices[0]..] > text[suffix_indices[1]..] {
            return false;
        }
    }

    true
}

proptest! {
    // default is 256 and I'd like some more test cases that need to pass
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn correctness_random_texts(text in prop::collection::vec(any::<u8>(), 0..1000)) {
        let maybe_suffix_array = SaisBuilder::new().construct_suffix_array(&text);

        prop_assert!(is_suffix_array(&maybe_suffix_array, &text));
    }

    #[test]
    fn correctness_lorem_ipsum_tests(
        text in (0..300usize).prop_map(|num_words|lipsum_words_with_rng(rand::thread_rng(), num_words))
    ) {
        let maybe_suffix_array = SaisBuilder::new().construct_suffix_array(text.as_bytes());
        prop_assert!(is_suffix_array(&maybe_suffix_array, text.as_bytes()));
    }

}
