use proptest::prelude::*;

use sais_drum::SaisBuilder;

// example from
// https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
static ABC_TEXT: &[u8] = b"ababcabcabba";

#[test]
fn whole_algorithm_u8_abc_text() {
    let result = SaisBuilder::new().construct_suffix_array(ABC_TEXT);
    let expected_suffix_array = [11, 0, 8, 5, 2, 10, 1, 9, 6, 3, 7, 4];

    assert!(is_suffix_array(&result, ABC_TEXT));
    assert_eq!(result, expected_suffix_array);
}

#[test]
fn whole_algorithm_short_texts() {
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
fn whole_algorithm_no_lms_mini_text() {
    let text = [0u8, 1];
    let suffix_array = SaisBuilder::new().construct_suffix_array(&text);

    assert_eq!(suffix_array, [0, 1]);
}

#[test]
fn whole_algorithm_one_lms_mini_text() {
    let text = b"424";
    let suffix_array = SaisBuilder::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [1, 2, 0]);
}

#[test]
fn whole_algorithm_two_lms_mini_text() {
    let text = b"yxyxy";
    let suffix_array = SaisBuilder::new().construct_suffix_array(text);

    assert_eq!(suffix_array, [3, 1, 4, 2, 0]);
}

fn is_suffix_array(maybe_suffix_array: &[usize], text: &[u8]) -> bool {
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
    #[test]
    fn whole_algorithm_correctness_random_texts(text in prop::collection::vec(any::<u8>(), 0..10_000)) {
        let maybe_suffix_array = SaisBuilder::new().construct_suffix_array(&text);

        prop_assert!(is_suffix_array(&maybe_suffix_array, &text));
    }
}
