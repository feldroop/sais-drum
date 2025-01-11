use std::{iter, sync::LazyLock};

use super::*;

// ------------------------------ ABC TEXT ------------------------------
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
    LazyLock::new(|| scan_for_counts_and_s_l_types(ABC_TEXT, 255));
static ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES: &[usize] = &[8, 5, 2];

#[test]
fn scan_for_counts_and_s_l_types_u8_abc_text() {
    assert_eq!(
        ABC_TEXT_METADATA.is_s_type,
        bitvec::bits![1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1]
    );

    assert_eq!(ABC_TEXT_METADATA.char_counts, *ABC_TEXT_EXPECTED_COUNTS);
}

#[test]
fn bucket_indices_from_counts_u8_abc_text() {
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
fn lms_substring_sorting_u8_abc_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; ABC_TEXT.len()];

    let num_lms_chars = place_text_order_lms_indices_into_buckets(
        &mut suffix_array_buffer,
        &ABC_TEXT_METADATA,
        ABC_TEXT,
    );
    assert_eq!(num_lms_chars, 3);

    let mut expected_suffix_array_buffer = vec![NONE_VALUE; ABC_TEXT.len()];
    expected_suffix_array_buffer[2] = 8;
    expected_suffix_array_buffer[3] = 5;
    expected_suffix_array_buffer[4] = 2;

    // the order of LMS indices inside the buckets is actually arbitrary
    assert_eq!(suffix_array_buffer, expected_suffix_array_buffer);

    induce_to_sort_lms_substrings(
        &mut suffix_array_buffer,
        &ABC_TEXT_METADATA.char_counts,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT,
    );

    assert_eq!(
        &suffix_array_buffer[(ABC_TEXT.len() - 3)..],
        ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES
    );
}

#[test]
fn create_reduced_text_u8_abc_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; ABC_TEXT.len()];
    suffix_array_buffer[(ABC_TEXT.len() - 3)..]
        .copy_from_slice(ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES);

    let (reduced_text_metadata, vacant_buffer) = create_reduced_text(
        3,
        &mut suffix_array_buffer,
        &ABC_TEXT_METADATA.is_s_type,
        ABC_TEXT,
    );

    assert_eq!(reduced_text_metadata.num_different_names, 2);
    assert_eq!(reduced_text_metadata.backtransformation_table, [2, 5, 8]);
    assert_eq!(reduced_text_metadata.data, [1, 1, 0]); // mistake in the example slides
    assert_eq!(vacant_buffer.len(), ABC_TEXT.len() - 6); // mistake in the example slides
}

#[test]
fn lms_substrings_are_unequal_u8_abc_text() {
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

// ------------------------------ NO LMS TEXT ------------------------------
// starts with S-type, contains no LMS chars except for the virtual sentinel
static NO_LMS_MINI_TEXT: &[u16] = &[0, 1];
static NO_LMS_MINI_TEXT_METADATA: LazyLock<TextMetadata> =
    LazyLock::new(|| scan_for_counts_and_s_l_types(NO_LMS_MINI_TEXT, 1));
static EMPTY_SLICE: &[usize] = &[];

#[test]
fn lms_substring_sorting_u16_no_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; NO_LMS_MINI_TEXT.len()];

    let num_lms_chars = place_text_order_lms_indices_into_buckets(
        &mut suffix_array_buffer,
        &NO_LMS_MINI_TEXT_METADATA,
        NO_LMS_MINI_TEXT,
    );
    assert_eq!(num_lms_chars, 0);

    // the order of LMS indices inside the buckets is actually arbitrary
    assert_eq!(suffix_array_buffer, [NONE_VALUE; NO_LMS_MINI_TEXT.len()]);

    // nothing really to check here, only that it doesn't panic
    induce_to_sort_lms_substrings(
        &mut suffix_array_buffer,
        &NO_LMS_MINI_TEXT_METADATA.char_counts,
        &NO_LMS_MINI_TEXT_METADATA.is_s_type,
        NO_LMS_MINI_TEXT,
    );
}

#[test]
fn create_reduced_text_u16_no_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; NO_LMS_MINI_TEXT.len()];

    let (reduced_text_metadata, vacant_buffer) = create_reduced_text(
        0,
        &mut suffix_array_buffer,
        &NO_LMS_MINI_TEXT_METADATA.is_s_type,
        NO_LMS_MINI_TEXT,
    );

    assert_eq!(reduced_text_metadata.num_different_names, 0);
    assert_eq!(reduced_text_metadata.backtransformation_table, EMPTY_SLICE);
    assert_eq!(reduced_text_metadata.data, EMPTY_SLICE);
    assert_eq!(vacant_buffer.len(), 2);
}

// ------------------------------ ONE LMS TEXT ------------------------------
// starts with L-type, contains exactly one LMS char except for the virtual sentinel
static ONE_LMS_MINI_TEXT: &[u32] = &[1, 0, 1];
static ONE_LMS_MINI_TEXT_METADATA: LazyLock<TextMetadata> =
    LazyLock::new(|| scan_for_counts_and_s_l_types(ONE_LMS_MINI_TEXT, 1));
static ONE_LMS_MINI_TEXT_LMS_CHARS: &[usize] = &[1];

#[test]
fn lms_substring_sorting_u32_one_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; ONE_LMS_MINI_TEXT.len()];

    let num_lms_chars = place_text_order_lms_indices_into_buckets(
        &mut suffix_array_buffer,
        &ONE_LMS_MINI_TEXT_METADATA,
        ONE_LMS_MINI_TEXT,
    );
    assert_eq!(num_lms_chars, 1);

    // the order of LMS indices inside the buckets is actually arbitrary
    assert_eq!(suffix_array_buffer, [1, NONE_VALUE, NONE_VALUE]);

    induce_to_sort_lms_substrings(
        &mut suffix_array_buffer,
        &ONE_LMS_MINI_TEXT_METADATA.char_counts,
        &ONE_LMS_MINI_TEXT_METADATA.is_s_type,
        ONE_LMS_MINI_TEXT,
    );

    assert_eq!(&suffix_array_buffer[2..], ONE_LMS_MINI_TEXT_LMS_CHARS);
}

#[test]
fn create_reduced_text_u32_one_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; ONE_LMS_MINI_TEXT.len()];
    suffix_array_buffer[2] = 1;

    let (reduced_text_metadata, vacant_buffer) = create_reduced_text(
        1,
        &mut suffix_array_buffer,
        &ONE_LMS_MINI_TEXT_METADATA.is_s_type,
        ONE_LMS_MINI_TEXT,
    );

    assert_eq!(reduced_text_metadata.data, [0]);
    assert_eq!(reduced_text_metadata.num_different_names, 1);
    assert_eq!(reduced_text_metadata.backtransformation_table, [1]);
    assert_eq!(vacant_buffer.len(), 1);
}

// ------------------------------ TWO LMS TEXT ------------------------------
// starts with L-type, contains exactly two LMS chars except for the virtual sentinel,
// LMS substrings are compared until the virtual sentinel is the difference maker
static TWO_LMS_MINI_TEXT: &[u64] = &[1, 0, 1, 0, 1];
static TWO_LMS_MINI_TEXT_METADATA: LazyLock<TextMetadata> =
    LazyLock::new(|| scan_for_counts_and_s_l_types(TWO_LMS_MINI_TEXT, 1));
static TWO_LMS_MINI_TEXT_LMS_CHARS: &[usize] = &[3, 1];

#[test]
fn lms_substring_sorting_u64_two_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; TWO_LMS_MINI_TEXT.len()];

    let num_lms_chars = place_text_order_lms_indices_into_buckets(
        &mut suffix_array_buffer,
        &TWO_LMS_MINI_TEXT_METADATA,
        TWO_LMS_MINI_TEXT,
    );
    assert_eq!(num_lms_chars, 2);

    // the order of LMS indices inside the buckets is actually arbitrary
    assert_eq!(
        suffix_array_buffer,
        [3, 1, NONE_VALUE, NONE_VALUE, NONE_VALUE]
    );

    induce_to_sort_lms_substrings(
        &mut suffix_array_buffer,
        &TWO_LMS_MINI_TEXT_METADATA.char_counts,
        &TWO_LMS_MINI_TEXT_METADATA.is_s_type,
        TWO_LMS_MINI_TEXT,
    );

    assert_eq!(&suffix_array_buffer[3..], TWO_LMS_MINI_TEXT_LMS_CHARS);
}

#[test]
fn create_reduced_text_u64_two_lms_mini_text() {
    let mut suffix_array_buffer = vec![NONE_VALUE; TWO_LMS_MINI_TEXT.len()];
    suffix_array_buffer[3..].copy_from_slice(TWO_LMS_MINI_TEXT_LMS_CHARS);

    let (reduced_text_metadata, vacant_buffer) = create_reduced_text(
        2,
        &mut suffix_array_buffer,
        &TWO_LMS_MINI_TEXT_METADATA.is_s_type,
        TWO_LMS_MINI_TEXT,
    );

    assert_eq!(reduced_text_metadata.num_different_names, 2);
    assert_eq!(reduced_text_metadata.backtransformation_table, &[1, 3]);
    assert_eq!(reduced_text_metadata.data, [1, 0]);
    assert_eq!(vacant_buffer.len(), 1);
}

#[test]
fn lms_substrings_are_unequal_u64_two_lms_mini_text() {
    assert!(lms_substrings_are_unequal(
        1,
        3,
        &TWO_LMS_MINI_TEXT_METADATA.is_s_type,
        TWO_LMS_MINI_TEXT
    ));
}
