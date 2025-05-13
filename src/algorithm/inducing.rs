use super::buckets::{
    iter_bucket_borders, iter_bucket_borders_rev, write_bucket_end_indices_into_buffer,
};
use super::is_lms_type;
use crate::{Character, NONE_VALUE};

use bitvec::slice::BitSlice;

// after this, the sorted LMS indices (by LMS substrings) are at the end of suffix_array_buffer
pub fn induce_to_sort_lms_substrings<C: Character>(
    suffix_array_buffer: &mut [usize],
    bucket_start_indices: &[usize],
    bucket_indices_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    induce_from_virtual_sentinel(suffix_array_buffer, bucket_indices_buffer, text);

    for (start, end) in iter_bucket_borders(bucket_start_indices, text.len()) {
        induce_range_left_to_right(
            start..end,
            suffix_array_buffer,
            bucket_indices_buffer,
            is_s_type,
            text,
        );
    }

    write_bucket_end_indices_into_buffer(bucket_start_indices, bucket_indices_buffer, text.len());

    let mut write_index = suffix_array_buffer.len() - 1;
    for (start, end) in iter_bucket_borders_rev(bucket_start_indices, text.len()) {
        induce_range_right_to_left_and_write_lms_indices_to_end(
            start..end,
            suffix_array_buffer,
            bucket_indices_buffer,
            is_s_type,
            text,
            &mut write_index,
        );
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}

pub fn induce_to_finalize_suffix_array<C: Character>(
    suffix_array_buffer: &mut [usize],
    bucket_start_indices: &[usize],
    bucket_indices_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    induce_from_virtual_sentinel(suffix_array_buffer, bucket_indices_buffer, text);

    for (start, end) in iter_bucket_borders(bucket_start_indices, text.len()) {
        induce_range_left_to_right(
            start..end,
            suffix_array_buffer,
            bucket_indices_buffer,
            is_s_type,
            text,
        );
    }

    write_bucket_end_indices_into_buffer(bucket_start_indices, bucket_indices_buffer, text.len());

    for (start, end) in iter_bucket_borders_rev(bucket_start_indices, text.len()) {
        induce_range_right_to_left(
            start..end,
            suffix_array_buffer,
            bucket_indices_buffer,
            is_s_type,
            text,
        );
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}

// the virtual sentinel would normally be at first position of the suffix array
fn induce_from_virtual_sentinel<C: Character>(
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    text: &[C],
) {
    let last_suffix_index = text.len() - 1;
    induce_l_type(
        last_suffix_index,
        suffix_array_buffer,
        bucket_indices_buffer,
        text,
    );
}

fn induce_range_left_to_right<C: Character>(
    index_range: impl Iterator<Item = usize>,
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    for suffix_array_index in index_range {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == NONE_VALUE || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        induce_l_type(
            suffix_index - 1,
            suffix_array_buffer,
            bucket_indices_buffer,
            text,
        );
    }
}

// rev() will be called on the index range
fn induce_range_right_to_left<C: Character>(
    index_range: impl DoubleEndedIterator<Item = usize>,
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    for suffix_array_index in index_range.rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 || !is_s_type[suffix_index - 1] {
            continue;
        }

        induce_s_type(
            suffix_index - 1,
            suffix_array_buffer,
            bucket_indices_buffer,
            text,
        );
    }
}

// rev() will be called on the index range
fn induce_range_right_to_left_and_write_lms_indices_to_end<C: Character>(
    index_range: impl DoubleEndedIterator<Item = usize>,
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
    write_index: &mut usize,
) {
    for suffix_array_index in index_range.rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 {
            continue;
        }

        // the LMS suffixes only induce L-type suffixes, which we are not interested in
        // instead, we prepare for creation of reduced text by moving all of the
        // LMS indices to the back of the array (now sorted by LMS substrings)
        if is_lms_type(suffix_index, is_s_type) {
            suffix_array_buffer[*write_index] = suffix_index;
            *write_index -= 1;
            continue;
        }

        if !is_s_type[suffix_index - 1] {
            continue;
        }

        induce_s_type(
            suffix_index - 1,
            suffix_array_buffer,
            bucket_indices_buffer,
            text,
        );
    }
}

fn induce_l_type<C: Character>(
    target_suffix_index: usize,
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    text: &[C],
) {
    let induced_suffix_first_char = text[target_suffix_index];
    let induced_suffix_bucket_start_index =
        &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

    suffix_array_buffer[*induced_suffix_bucket_start_index] = target_suffix_index;
    *induced_suffix_bucket_start_index += 1;
}

fn induce_s_type<C: Character>(
    target_suffix_index: usize,
    suffix_array_buffer: &mut [usize],
    bucket_indices_buffer: &mut [usize],
    text: &[C],
) {
    let induced_suffix_first_char = text[target_suffix_index];
    let induced_suffix_bucket_end_index =
        &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

    suffix_array_buffer[*induced_suffix_bucket_end_index] = target_suffix_index;

    // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
    // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
    *induced_suffix_bucket_end_index = induced_suffix_bucket_end_index.saturating_sub(1);
}
