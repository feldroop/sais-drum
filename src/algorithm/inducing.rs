use super::buckets;
use super::text_analysis::TextMetadata;
use crate::{Character, IndexStorage};

use bitvec::slice::BitSlice;
use num_traits::NumCast;

// after this, the sorted LMS indices (by LMS substrings) are at the end of suffix_array_buffer
pub fn induce_to_sort_lms_substrings<C: Character, I: IndexStorage>(
    suffix_array_buffer: &mut [I],
    bucket_start_indices: &[I],
    working_bucket_indices_buffer: &mut [I],
    text_metadata: &TextMetadata<I>,
    text: &[C],
) {
    working_bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    induce_from_virtual_sentinel(suffix_array_buffer, working_bucket_indices_buffer, text);

    for (start, end) in buckets::iter_bucket_borders(bucket_start_indices, text.len()) {
        induce_range_left_to_right(
            num::range(start, end),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text_metadata.is_s_type,
            text,
        );
    }

    buckets::write_bucket_end_indices_into_buffer(
        bucket_start_indices,
        working_bucket_indices_buffer,
        text.len(),
    );

    let mut write_index = suffix_array_buffer.len() - 1;
    for (start, end) in buckets::iter_bucket_borders_rev(bucket_start_indices, text.len()) {
        induce_range_right_to_left_and_write_lms_indices_to_end(
            num::range(start, end),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text_metadata,
            text,
            &mut write_index,
        );
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}

pub fn induce_to_finalize_suffix_array<C: Character, I: IndexStorage>(
    suffix_array_buffer: &mut [I],
    bucket_start_indices: &[I],
    working_bucket_indices_buffer: &mut [I],
    text_metadata: &TextMetadata<I>,
    text: &[C],
) {
    working_bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    induce_from_virtual_sentinel(suffix_array_buffer, working_bucket_indices_buffer, text);

    for (start, end) in buckets::iter_bucket_borders(bucket_start_indices, text.len()) {
        induce_range_left_to_right(
            num::range(start, end),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text_metadata.is_s_type,
            text,
        );
    }

    buckets::write_bucket_end_indices_into_buffer(
        bucket_start_indices,
        working_bucket_indices_buffer,
        text.len(),
    );

    for (start, end) in buckets::iter_bucket_borders_rev(bucket_start_indices, text.len()) {
        induce_range_right_to_left(
            num::range(start, end),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text_metadata.is_s_type,
            text,
        );
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}

// the virtual sentinel would normally be at first position of the suffix array
fn induce_from_virtual_sentinel<C: Character, I: IndexStorage>(
    suffix_array_buffer: &mut [I],
    working_bucket_indices_buffer: &mut [I],
    text: &[C],
) {
    let last_suffix_index = <I as NumCast>::from(text.len() - 1).unwrap();
    induce_l_type(
        last_suffix_index,
        suffix_array_buffer,
        working_bucket_indices_buffer,
        text,
    );
}

fn induce_range_left_to_right<C: Character, I: IndexStorage>(
    index_range: impl Iterator<Item = I>,
    suffix_array_buffer: &mut [I],
    working_bucket_indices_buffer: &mut [I],
    is_s_type: &BitSlice<I>,
    text: &[C],
) {
    for suffix_array_index in index_range {
        let suffix_index = suffix_array_buffer[suffix_array_index.as_()];

        if suffix_index == I::max_value()
            || suffix_index == I::zero()
            || is_s_type[suffix_index.as_() - 1]
        {
            continue;
        }

        induce_l_type(
            suffix_index - I::one(),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text,
        );
    }
}

// rev() will be called on the index range
fn induce_range_right_to_left<C: Character, I: IndexStorage>(
    index_range: impl DoubleEndedIterator<Item = I>,
    suffix_array_buffer: &mut [I],
    working_bucket_indices_buffer: &mut [I],
    is_s_type: &BitSlice<I>,
    text: &[C],
) {
    for suffix_array_index in index_range.rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index.as_()];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == I::zero() || !is_s_type[suffix_index.as_() - 1] {
            continue;
        }

        induce_s_type(
            suffix_index - I::one(),
            suffix_array_buffer,
            working_bucket_indices_buffer,
            text,
        );
    }
}

// rev() will be called on the index range
fn induce_range_right_to_left_and_write_lms_indices_to_end<C: Character, I: IndexStorage>(
    index_range: impl DoubleEndedIterator<Item = I>,
    suffix_array_buffer: &mut [I],
    bucket_indices_buffer: &mut [I],
    text_metadata: &TextMetadata<I>,
    text: &[C],
    write_index: &mut usize,
) {
    for suffix_array_index in index_range.rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index.as_()];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == I::zero() {
            continue;
        }

        // the LMS suffixes only induce L-type suffixes, which we are not interested in
        // instead, we prepare for creation of reduced text by moving all of the
        // LMS indices to the back of the array (now sorted by LMS substrings)
        if text_metadata.is_lms_type(suffix_index) {
            suffix_array_buffer[*write_index] = suffix_index;
            *write_index -= 1;
            continue;
        }

        if !text_metadata.is_s_type[suffix_index.as_() - 1] {
            continue;
        }

        induce_s_type(
            suffix_index - I::one(),
            suffix_array_buffer,
            bucket_indices_buffer,
            text,
        );
    }
}

fn induce_l_type<C: Character, I: IndexStorage>(
    target_suffix_index: I,
    suffix_array_buffer: &mut [I],
    working_bucket_indices_buffer: &mut [I],
    text: &[C],
) {
    let induced_suffix_first_char = text[target_suffix_index.as_()];
    let induced_suffix_bucket_start_index =
        &mut working_bucket_indices_buffer[induced_suffix_first_char.rank()];

    suffix_array_buffer[induced_suffix_bucket_start_index.as_()] = target_suffix_index;
    *induced_suffix_bucket_start_index = *induced_suffix_bucket_start_index + I::one();
}

fn induce_s_type<C: Character, I: IndexStorage>(
    target_suffix_index: I,
    suffix_array_buffer: &mut [I],
    working_bucket_indices_buffer: &mut [I],
    text: &[C],
) {
    let induced_suffix_first_char = text[target_suffix_index.as_()];
    let induced_suffix_bucket_end_index =
        &mut working_bucket_indices_buffer[induced_suffix_first_char.rank()];

    suffix_array_buffer[induced_suffix_bucket_end_index.as_()] = target_suffix_index;

    // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
    // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
    *induced_suffix_bucket_end_index = induced_suffix_bucket_end_index.saturating_sub(I::one());
}
