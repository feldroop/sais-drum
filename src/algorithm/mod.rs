// #[cfg(test)]
// mod tests;

use crate::Character;

use bitvec::prelude::*;

use std::cmp::{self, Ordering};

// the text must always be smaller than this value
pub(crate) const NONE_VALUE: usize = usize::MAX;
const WRAPPING_ZERO_DECREMENT_RESULT: usize = usize::MAX;

// TODO SaisConfig for configuration of this algorithm that is not the public API

// expects suffix array buffer to be filled with EMPTY_VALUE and of the same length as text
pub fn suffix_array_induced_sort<C: Character>(
    text: &[C],
    max_char: C,
    suffix_array_buffer: &mut [usize],
    extra_buffer: Option<&mut [usize]>,
) {
    if text.is_empty() {
        return;
    }

    // this buffer will contain the bucket_start_indices after being initialized with counts
    // if extra_buffer is sufficient, this will actually stay empty
    let mut bucket_indices_buffer1 = Vec::new();

    let (char_counts, remaining_extra_buffer) = reuse_extra_buffer_or_allocate_owned(
        &mut bucket_indices_buffer1,
        extra_buffer,
        max_char.rank() + 1,
    );

    // TODO maybe skip this scan in recursion
    let is_s_type = scan_for_counts_and_s_l_types(text, char_counts);

    let bucket_start_indices = char_counts;
    counts_into_bucket_start_indices(bucket_start_indices);

    // varies between end and start indices, and is overwritten in LMS index placement and induction
    let mut bucket_indices_buffer2 = Vec::new();

    let (bucket_indices_working_buffer, vacant_buffer1) = reuse_extra_buffer_or_allocate_owned(
        &mut bucket_indices_buffer2,
        remaining_extra_buffer,
        max_char.rank() + 1,
    );

    write_bucket_end_indices_into_buffer(
        bucket_start_indices,
        bucket_indices_working_buffer,
        text.len(),
    );

    let num_lms_chars = place_text_order_lms_indices_into_buckets(
        suffix_array_buffer,
        bucket_indices_working_buffer,
        &is_s_type,
        text,
    );

    induce_to_sort_lms_substrings(
        suffix_array_buffer,
        bucket_start_indices,
        bucket_indices_working_buffer,
        &is_s_type,
        text,
    );

    // move sorted lms indices to front of the buffer
    let (rest, lms_indices) =
        suffix_array_buffer.split_at_mut(suffix_array_buffer.len() - num_lms_chars);
    rest[..num_lms_chars].copy_from_slice(lms_indices);

    let num_different_names =
        create_reduced_text(num_lms_chars, suffix_array_buffer, &is_s_type, text);

    let (rest, reduced_text) =
        suffix_array_buffer.split_at_mut(suffix_array_buffer.len() - num_lms_chars);

    // put reduced suffix array buffer at the end of the buffer instead of the middle,
    // this make the implementation later a bit simpler (when placing sorted LMS indices into buckets)
    let (reduced_text_suffix_array_buffer, vacant_buffer2) = rest.split_at_mut(num_lms_chars);

    if num_different_names == num_lms_chars {
        directly_construct_suffix_array(reduced_text, reduced_text_suffix_array_buffer);
    } else {
        reduced_text_suffix_array_buffer.fill(NONE_VALUE);

        let larger_vacant_buffer = choose_larger_vacant_buffer(vacant_buffer1, vacant_buffer2);

        suffix_array_induced_sort(
            reduced_text,
            num_different_names - 1,
            reduced_text_suffix_array_buffer,
            Some(larger_vacant_buffer),
        );
    };

    let backtransformation_table = reduced_text; // reuse this buffer
    create_backtransformation_table(backtransformation_table, &is_s_type);

    backtransform_into_original_text_lms_indices(
        reduced_text_suffix_array_buffer,
        backtransformation_table,
    );

    place_sorted_lms_indices_into_buckets(
        suffix_array_buffer,
        num_lms_chars,
        bucket_start_indices,
        bucket_indices_working_buffer,
        text,
    );

    induce_to_finalize_suffix_array(
        suffix_array_buffer,
        bucket_start_indices,
        bucket_indices_working_buffer,
        &is_s_type,
        text,
    );
}

fn scan_for_counts_and_s_l_types<C: Character>(text: &[C], char_counts: &mut [usize]) -> BitVec {
    let mut is_s_type = BitVec::repeat(true, text.len() + 1);

    // sentinel is by definiton S-type and the smallest character
    let mut current_char_compared_to_previous = Ordering::Greater;

    for (text_index, char) in text.iter().enumerate().rev() {
        char_counts[char.rank()] += 1;

        let current_char_is_s_type = match current_char_compared_to_previous {
            Ordering::Less => true,
            Ordering::Equal => is_s_type[text_index + 1],
            Ordering::Greater => false,
        };

        is_s_type.set(text_index, current_char_is_s_type);

        if text_index == 0 {
            break;
        }

        current_char_compared_to_previous = text[text_index - 1].cmp(&text[text_index])
    }

    is_s_type
}

// returns number of lms chars, without sentinel
fn place_text_order_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    bucket_end_indices: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) -> usize {
    let mut num_lms_chars = 0;

    for (text_index, char) in text.iter().enumerate().skip(1) {
        if !is_lms_type(text_index, is_s_type) {
            continue;
        }

        num_lms_chars += 1;

        let bucket_end_index = &mut bucket_end_indices[char.rank()];

        suffix_array_buffer[*bucket_end_index] = text_index;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *bucket_end_index = bucket_end_index.saturating_sub(1);
    }

    num_lms_chars
}

// expects sorted LMS indices (i.e reduced text suffix array) at the front of suffix_array_buffer
fn place_sorted_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    num_lms_chars: usize,
    bucket_start_indices: &[usize],
    bucket_indices_buffer: &mut [usize],
    text: &[C],
) {
    write_bucket_end_indices_into_buffer(bucket_start_indices, bucket_indices_buffer, text.len());

    // this works, because the sorted LMS indices are sorted, have the same order in the full
    // suffix_array_buffer as they have before (the sorted order!), i.e. we won't override
    // the part of the buffer we are iteraring through
    for index_of_sorted_lms_indices in (0..num_lms_chars).rev() {
        let lms_char_index = suffix_array_buffer[index_of_sorted_lms_indices];
        let lms_char = text[lms_char_index];
        let bucket_end_index = &mut bucket_indices_buffer[lms_char.rank()];

        suffix_array_buffer[*bucket_end_index] = lms_char_index;

        // wrapping sub used, because the last placement of the first bucket (index 0) might underflow
        *bucket_end_index = bucket_end_index.wrapping_sub(1);
    }

    // fill the rest of the array with NONE_VALUE
    // TODO maybe this could be implemented more efficiently by also doing it in the above loop
    for (&bucket_start_index, &mut bucket_end_index_after_lms_placement) in
        bucket_start_indices.iter().zip(bucket_indices_buffer)
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

// after this, the sorted LMS indices (by LMS substrings) are at the end of suffix_array_buffer
fn induce_to_sort_lms_substrings<C: Character>(
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

fn induce_to_finalize_suffix_array<C: Character>(
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

// reduced text is written to the end of the suffix array buffer
// LMS indices at front of input suffix_array_buffer should be sorted according to
// their LMS substrings (not necessarily according to their whole LMS suffixes)
// returns number of different names in the text
fn create_reduced_text<C: Character>(
    num_lms_chars: usize,
    suffix_array_buffer: &mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) -> usize {
    if num_lms_chars == 0 {
        return 0;
    }

    let (rest_of_suffix_array_buffer, reduced_text_placement_buffer) =
        suffix_array_buffer.split_at_mut(text.len() / 2);

    reduced_text_placement_buffer.fill(NONE_VALUE);

    let sorted_lms_substring_indices = &mut rest_of_suffix_array_buffer[..num_lms_chars];
    let mut current_name = 0;

    for index_of_sorted_lms_substring_indices in 0..sorted_lms_substring_indices.len() - 1 {
        let curr_lms_substring_index =
            sorted_lms_substring_indices[index_of_sorted_lms_substring_indices];

        // TODO write comment on why this is sound
        let placement_index = curr_lms_substring_index >> 1;

        reduced_text_placement_buffer[placement_index] = current_name;

        let next_lms_substring_index =
            sorted_lms_substring_indices[index_of_sorted_lms_substring_indices + 1];
        // if current lms substring is not last and != next lms substring
        if lms_substrings_are_unequal(
            curr_lms_substring_index,
            next_lms_substring_index,
            is_s_type,
            text,
        ) {
            current_name += 1;
        }
    }

    let last_placement_index = *sorted_lms_substring_indices.last().unwrap() >> 1;
    reduced_text_placement_buffer[last_placement_index] = current_name;

    let mut write_index = reduced_text_placement_buffer.len() - 1;
    for read_index in (0..reduced_text_placement_buffer.len()).rev() {
        let maybe_lms_substring_name = reduced_text_placement_buffer[read_index];

        if maybe_lms_substring_name != NONE_VALUE {
            reduced_text_placement_buffer[write_index] = maybe_lms_substring_name;
            write_index -= 1;
        }
    }

    current_name + 1
}

// base case of recursion. this works, because the reduced text exclusively contains unique characters
fn directly_construct_suffix_array(
    reduced_text: &mut [usize],
    reduced_text_suffix_array_buffer: &mut [usize],
) {
    for (reduced_text_suffix_index, &reduced_text_char) in reduced_text.iter().enumerate() {
        reduced_text_suffix_array_buffer[reduced_text_char] = reduced_text_suffix_index;
    }
}

fn create_backtransformation_table(backtransformation_table: &mut [usize], is_s_type: &BitSlice) {
    let mut write_index = 0;

    for text_index in 1..is_s_type.len() - 1 {
        if is_lms_type(text_index, is_s_type) {
            backtransformation_table[write_index] = text_index;
            write_index += 1;
        }
    }
}

fn backtransform_into_original_text_lms_indices(
    reduced_text_suffix_array_buffer: &mut [usize],
    backtransformation_table: &[usize],
) {
    for reduced_text_index in reduced_text_suffix_array_buffer {
        *reduced_text_index = backtransformation_table[*reduced_text_index];
    }
}

// assumes index > 0
#[inline]
fn is_lms_type(index: usize, is_s_type: &BitSlice) -> bool {
    is_s_type[index] && !is_s_type[index - 1]
}

// returns (a,b) where a is a buffer of length num_buckets with all values set to 0 and b maybe another buffer
fn reuse_extra_buffer_or_allocate_owned<'a>(
    owned_buffer: &'a mut Vec<usize>,
    extra_buffer: Option<&'a mut [usize]>,
    num_buckets: usize,
) -> (&'a mut [usize], Option<&'a mut [usize]>) {
    if extra_buffer.is_some() && extra_buffer.as_ref().unwrap().len() >= num_buckets {
        let (buffer, rest) = extra_buffer.unwrap().split_at_mut(num_buckets);
        buffer.fill(0);
        return (buffer, Some(rest));
    }

    owned_buffer.resize(num_buckets, 0);
    (owned_buffer, extra_buffer)
}

// inclusive index, the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
fn counts_into_bucket_start_indices(char_counts: &mut [usize]) {
    let mut sum = 0;

    for value in char_counts.iter_mut() {
        let temp = sum;
        sum += *value;
        *value = temp;
    }
}

// inclusive index, except for empty buckets, there the end index is the start index - 1
// the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
// overwrites given bucket end indices buffer
fn write_bucket_end_indices_into_buffer(
    bucket_start_indices: &[usize],
    bucket_indices_buffer: &mut [usize],
    text_len: usize,
) {
    let alphabet_size = bucket_start_indices.len();
    for (bucket_end_index, next_bucket_start_index) in bucket_indices_buffer[..alphabet_size - 1]
        .iter_mut()
        .zip(&bucket_start_indices[1..])
    {
        *bucket_end_index = next_bucket_start_index.wrapping_sub(1);
    }

    // edge case for when the last character does not appear in the text
    let num_buckets = bucket_start_indices.len();
    *bucket_indices_buffer.last_mut().unwrap() = if text_len == 1
        || num_buckets == 1
        || bucket_start_indices[num_buckets - 1] != bucket_start_indices[num_buckets - 2]
    {
        text_len - 1
    } else {
        text_len - 2
    };
}

fn choose_larger_vacant_buffer<'a>(
    vacant_buffer1: Option<&'a mut [usize]>,
    vacant_buffer2: &'a mut [usize],
) -> &'a mut [usize] {
    if let Some(vacant_buffer1) = vacant_buffer1 {
        cmp::max_by(vacant_buffer1, vacant_buffer2, |buf1, buf2| {
            buf1.len().cmp(&buf2.len())
        })
    } else {
        vacant_buffer2
    }
}

// this assumes, that the two given indices are different. otherwise it might return true
// in some edge cases, when first_lms_substring_index == second_lms_substring_index
fn lms_substrings_are_unequal<C: Character>(
    first_lms_substring_index: usize,
    second_lms_substring_index: usize,
    is_s_type: &BitSlice,
    text: &[C],
) -> bool {
    let mut smaller_lms_substring_index =
        cmp::min(first_lms_substring_index, second_lms_substring_index);
    let mut larger_lms_substring_index =
        cmp::max(first_lms_substring_index, second_lms_substring_index);

    if text[smaller_lms_substring_index] != text[larger_lms_substring_index] {
        return true;
    }

    // this is in bounds, because every lms substring has at least length 2 (not counting virtual sentinel),
    // except for the sentinel itself, but it is handled as a virtual character, so it does never appear here
    smaller_lms_substring_index += 1;
    larger_lms_substring_index += 1;

    // lms-substrings are defined as equal if their length, their S/L-types and characters all match
    // in this loop, we only check the characters until one of the strings ends, because if
    // the characters match until position i, the types also match until position i-1. therefore,
    // the types might differ in the last iteration. that is why we check if both of the substrings are
    // terminated by an LMS-char at that positon and therefore are both S-type
    loop {
        if text[smaller_lms_substring_index] != text[larger_lms_substring_index] {
            return true;
        }

        let first_substring_is_over = is_lms_type(smaller_lms_substring_index, is_s_type);
        let second_substring_is_over = is_lms_type(larger_lms_substring_index, is_s_type);

        if first_substring_is_over || second_substring_is_over {
            return first_substring_is_over != second_substring_is_over;
        }

        smaller_lms_substring_index += 1;
        larger_lms_substring_index += 1;

        if larger_lms_substring_index == text.len() {
            // substrings are unequal, because one of them contains the unique sentinel
            return true;
        }
    }
}
