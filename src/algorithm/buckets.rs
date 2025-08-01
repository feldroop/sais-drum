use super::{NONE_VALUE, WRAPPING_ZERO_DECREMENT_RESULT, buckets, text_analysis::TextMetadata};
use crate::Character;

use std::iter;

// inclusive index, the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
pub fn counts_into_bucket_start_indices(persistent_bucket_start_indices_buffer: &mut [usize]) {
    let mut sum = 0;

    for value in persistent_bucket_start_indices_buffer.iter_mut() {
        let temp = sum;
        sum += *value;
        *value = temp;
    }
}

// inclusive index, except for empty buckets, there the end index is the start index - 1
// the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
// overwrites given bucket end indices buffer
pub fn write_bucket_end_indices_into_buffer(
    bucket_start_indices: &[usize],
    working_bucket_indices_buffer: &mut [usize],
    text_len: usize,
) {
    for (bucket_end_index, bucket_buffer_position) in
        iter_bucket_end_indices(bucket_start_indices, text_len).zip(working_bucket_indices_buffer)
    {
        *bucket_buffer_position = bucket_end_index;
    }
}

pub fn iter_bucket_end_indices(
    bucket_start_indices: &[usize],
    text_len: usize,
) -> impl Iterator<Item = usize> {
    // edge case for when the last character does not appear in the text
    let num_buckets = bucket_start_indices.len();
    let last_bucket_end_index = if text_len == 1
        || num_buckets == 1
        || bucket_start_indices[num_buckets - 1] != bucket_start_indices[num_buckets - 2]
    {
        text_len - 1
    } else {
        text_len - 2
    };

    bucket_start_indices[1..]
        .iter()
        .map(|next_bucket_start_index| next_bucket_start_index.wrapping_sub(1))
        .chain(iter::once(last_bucket_end_index))
}

// iterates over the borders of the buckets in the form of [start, one_behind_end)
pub fn iter_bucket_borders(
    bucket_start_indices: &[usize],
    text_len: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let next_bucket_start_indices = bucket_start_indices[1..]
        .iter()
        .copied()
        .chain(iter::once(text_len));

    bucket_start_indices
        .iter()
        .copied()
        .zip(next_bucket_start_indices)
}

// iterates over the borders of the buckets in the form of [start, one_behind_end), in reverse order
// this exists, because the chain method does not return an exact size iterator, so rev() cannot be called on it
pub fn iter_bucket_borders_rev(
    bucket_start_indices: &[usize],
    text_len: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let next_bucket_start_indices =
        iter::once(text_len).chain(bucket_start_indices[1..].iter().copied().rev());

    bucket_start_indices
        .iter()
        .copied()
        .rev()
        .zip(next_bucket_start_indices)
}

// returns number of lms chars, without sentinel
pub fn place_text_order_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    bucket_start_indices: &[usize],
    working_bucket_indices_buffer: &mut [usize],
    text: &[C],
    text_metadata: &TextMetadata,
) -> usize {
    buckets::write_bucket_end_indices_into_buffer(
        bucket_start_indices,
        working_bucket_indices_buffer,
        text.len(),
    );

    let mut num_lms_chars = 0;

    for (text_index, char) in text.iter().enumerate().skip(1) {
        // TODO maybe do this in initial scan
        if !text_metadata.is_lms_type(text_index) {
            continue;
        }

        num_lms_chars += 1;

        let bucket_end_index = &mut working_bucket_indices_buffer[char.rank()];

        suffix_array_buffer[*bucket_end_index] = text_index;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *bucket_end_index = bucket_end_index.saturating_sub(1);
    }

    num_lms_chars
}

// expects sorted LMS indices (i.e backtransformed reduced text suffix array) at the front of suffix_array_buffer
pub fn place_sorted_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    num_lms_chars: usize,
    persistent_bucket_start_indices_buffer: &[usize],
    working_bucket_indices_buffer: &mut [usize],
    text: &[C],
) {
    write_bucket_end_indices_into_buffer(
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
        text.len(),
    );

    // this works, because the LMS indices are sorted, have the same order in the full
    // suffix_array_buffer as they have before (the sorted order!), i.e. we won't override
    // the part of the buffer we are iteraring through
    for index_of_sorted_lms_indices in (0..num_lms_chars).rev() {
        let lms_char_index = suffix_array_buffer[index_of_sorted_lms_indices];
        let lms_char = text[lms_char_index];
        let bucket_end_index = &mut working_bucket_indices_buffer[lms_char.rank()];

        suffix_array_buffer[*bucket_end_index] = lms_char_index;

        // wrapping sub used, because the last placement of the first bucket (index 0) might underflow
        *bucket_end_index = bucket_end_index.wrapping_sub(1);
    }

    // fill the rest of the array with NONE_VALUE
    // TODO maybe this could be implemented more efficiently by also doing it in the above loop
    for (&bucket_start_index, &mut bucket_end_index_after_lms_placement) in
        persistent_bucket_start_indices_buffer
            .iter()
            .zip(working_bucket_indices_buffer)
    {
        // special case for the first bucket where a wrapping underflow might have happened
        if bucket_end_index_after_lms_placement == WRAPPING_ZERO_DECREMENT_RESULT
            || bucket_end_index_after_lms_placement < bucket_start_index
        {
            continue;
        }

        suffix_array_buffer[bucket_start_index..=bucket_end_index_after_lms_placement]
            .fill(NONE_VALUE);
    }
}
