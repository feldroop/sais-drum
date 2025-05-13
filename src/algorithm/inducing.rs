use super::{is_lms_type, write_bucket_end_indices_into_buffer};
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
    // ---------- LEFT TO RIGHT SCAN ----------
    bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    // virtual sentinel induction, it would normally be at first position of the suffix array
    let last_suffix_index = text.len() - 1;
    let last_suffix_char = text[last_suffix_index];
    let last_suffix_bucket_start_index = &mut bucket_indices_buffer[last_suffix_char.rank()];
    suffix_array_buffer[*last_suffix_bucket_start_index] = last_suffix_index;
    *last_suffix_bucket_start_index += 1;

    for suffix_array_index in 0..suffix_array_buffer.len() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == NONE_VALUE || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_start_index =
            &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_start_index] = suffix_index - 1;
        *induced_suffix_bucket_start_index += 1;
    }

    // ---------- RIGHT TO LEFT SCAN ----------
    write_bucket_end_indices_into_buffer(bucket_start_indices, bucket_indices_buffer, text.len());

    let mut write_index = suffix_array_buffer.len() - 1;

    for suffix_array_read_index in (0..suffix_array_buffer.len()).rev() {
        let suffix_index = suffix_array_buffer[suffix_array_read_index];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 {
            continue;
        }

        // the LMS suffixes only induce L-type suffixes, which we are not interested in
        // instead, we prepare for creation of reduced text by moving all of the
        // LMS indices to the back of the array (now sorted by LMS substrings)
        if is_lms_type(suffix_index, is_s_type) {
            suffix_array_buffer[write_index] = suffix_index;
            write_index -= 1;
            continue;
        }

        if !is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_end_index =
            &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_end_index] = suffix_index - 1;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *induced_suffix_bucket_end_index = induced_suffix_bucket_end_index.saturating_sub(1);
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
    // ---------- LEFT TO RIGHT SCAN ----------
    bucket_indices_buffer.copy_from_slice(bucket_start_indices);

    // virtual sentinel induction, it would normally be at first position of the suffix array
    let last_suffix_index = text.len() - 1;
    let last_suffix_char = text[last_suffix_index];
    let last_suffix_bucket_start_index = &mut bucket_indices_buffer[last_suffix_char.rank()];
    suffix_array_buffer[*last_suffix_bucket_start_index] = last_suffix_index;
    *last_suffix_bucket_start_index += 1;

    for suffix_array_index in 0..suffix_array_buffer.len() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == NONE_VALUE || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_start_index =
            &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_start_index] = suffix_index - 1;
        *induced_suffix_bucket_start_index += 1;
    }

    // ---------- RIGHT TO LEFT SCAN ----------
    write_bucket_end_indices_into_buffer(bucket_start_indices, bucket_indices_buffer, text.len());

    for suffix_array_index in (0..suffix_array_buffer.len()).rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 || !is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_end_index =
            &mut bucket_indices_buffer[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_end_index] = suffix_index - 1;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *induced_suffix_bucket_end_index = induced_suffix_bucket_end_index.saturating_sub(1);
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}
