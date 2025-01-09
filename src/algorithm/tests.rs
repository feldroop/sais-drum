use std::{iter, sync::LazyLock};

use super::*;

// example from
// https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
// LMS-chars                 *  *  *   *<- virtual sentinel
// S/L-types               SLSSLSSLSLLLS
static ABC_TEXT: &[u8] = b"ababcabcabba";

static ABC_TEXT_EXPECTED_COUNTS: LazyLock<Vec<usize>> = LazyLock::new(|| {
    let mut counts = vec![0usize; 256];
    counts[b'a' as usize] = 5;
    counts[b'b' as usize] = 5;
    counts[b'c' as usize] = 2;
    counts
});
static ABC_TEXT_METADATA: LazyLock<TextMetadata> =
    LazyLock::new(|| scan_for_counts_types_and_lms_chars(ABC_TEXT, 255));
static ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES: &[usize] = &[8, 2, 5];

#[test]
fn test_scan_for_counts_types_and_lms_chars_u8_abc_text() {
    assert_eq!(
        ABC_TEXT_METADATA.is_s_type,
        bitvec::bits![1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1]
    );

    assert_eq!(
        ABC_TEXT_METADATA.reverse_order_lms_char_indices,
        [12, 8, 5, 2]
    );
    assert_eq!(&ABC_TEXT_METADATA.char_counts, &*ABC_TEXT_EXPECTED_COUNTS);
}

#[test]
fn test_bucket_indices_from_counts_u8_abc_text() {
    let bucket_start_indices = bucket_start_indices_from_counts(&ABC_TEXT_METADATA.char_counts);
    let bucket_end_indices = bucket_end_indices_from_counts(&ABC_TEXT_METADATA.char_counts);

    let expected_bucket_start_indices: Vec<_> = iter::repeat_n(0usize, b'a' as usize + 1)
        .chain([5, 10])
        .chain(iter::repeat_n(12, 255 - b'c' as usize))
        .collect();

    let expected_bucket_end_indices: Vec<_> = iter::repeat_n(0usize, b'a' as usize)
        .chain([4, 9])
        .chain(iter::repeat_n(11, 256 - b'c' as usize))
        .collect();

    assert_eq!(bucket_start_indices, expected_bucket_start_indices);
    assert_eq!(bucket_end_indices, expected_bucket_end_indices);
}

#[test]
fn test_lms_substring_sorting_u8_abc_text() {
    let mut suffix_array_buffer = vec![usize::MAX; ABC_TEXT.len()];

    initialize_lms_indices_and_induce(
        &mut suffix_array_buffer,
        ABC_TEXT_METADATA
            .reverse_order_lms_char_indices
            .iter()
            .skip(1)
            .copied(),
        &ABC_TEXT_METADATA.char_counts,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT,
    );

    // different than in the example slides, because multiple results are possible
    // it would be better to check that all LMS substrings are sorted instead of a fixed result
    assert_eq!(
        suffix_array_buffer,
        vec![11, 0, 8, 2, 5, 10, 1, 9, 3, 6, 4, 7]
    );

    let sorted_lms_substring_indices =
        extract_sorted_lms_substring_indices(&ABC_TEXT_METADATA.is_s_type, &suffix_array_buffer);

    assert_eq!(
        sorted_lms_substring_indices,
        ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES
    );
}

#[test]
fn test_create_reduced_text_u8_abc_text() {
    let mut suffix_array_buffer = vec![usize::MAX; ABC_TEXT.len()];

    let reduced_text_metadata = create_reduced_text(
        ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES,
        &mut suffix_array_buffer,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT,
    );

    assert_eq!(reduced_text_metadata.data.len(), 3);
    assert_eq!(reduced_text_metadata.num_different_names, 2);
    assert_eq!(reduced_text_metadata.backtransformation_table, &[2, 5, 8]);
    assert_eq!(reduced_text_metadata.data, [1, 1, 0]); // mistake in the example slides
}

#[test]
fn test_lms_substrings_are_unequal_u8_abc_text() {
    assert!(lms_substrings_are_unequal(
        2,
        8,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT
    ));
    assert!(!lms_substrings_are_unequal(
        2,
        5,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT
    ));
}
