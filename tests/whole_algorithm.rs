use proptest::prelude::*;

use sais_drum::SaisBuilder;

// example from
// https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
static ABC_TEXT: &[u8] = b"ababcabcabba";

#[test]
fn test_whole_algorithm_u8_abc_text() {
    let naive_result = construct_suffix_array_naive(ABC_TEXT);
    let result = SaisBuilder::new().construct_suffix_array(ABC_TEXT);

    assert_eq!(naive_result, [11, 0, 8, 5, 2, 10, 1, 9, 6, 3, 7, 4]);
    assert_eq!(result, naive_result);
}

#[test]
fn test_whole_algorithm_short_texts() {
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
fn test_whole_algorithm_no_lms_mini_text() {
    let text = [0u8, 1];
    let _ = SaisBuilder::new().construct_suffix_array(&text);
}

fn construct_suffix_array_naive(text: &[u8]) -> Vec<usize> {
    let mut suffix_array: Vec<_> = (0..text.len()).collect();
    suffix_array.sort_unstable_by_key(|&index| &text[index..]);
    suffix_array
}

proptest! {
    #[test]
    fn doesnt_crash_short(text in prop::collection::vec(any::<u8>(), 0..100)) {
        let _ = SaisBuilder::new().construct_suffix_array(&text);
    }
}
