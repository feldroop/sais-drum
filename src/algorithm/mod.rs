// #[cfg(test)]
// mod tests;

mod buckets;
mod inducing;

use crate::Character;

use std::cmp::{self, Ordering};

use bitvec::prelude::*;

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
    buckets::counts_into_bucket_start_indices(bucket_start_indices);

    // varies between end and start indices, and is overwritten in LMS index placement and induction
    let mut bucket_indices_buffer2 = Vec::new();

    let (bucket_indices_working_buffer, vacant_buffer1) = reuse_extra_buffer_or_allocate_owned(
        &mut bucket_indices_buffer2,
        remaining_extra_buffer,
        max_char.rank() + 1,
    );

    buckets::write_bucket_end_indices_into_buffer(
        bucket_start_indices,
        bucket_indices_working_buffer,
        text.len(),
    );

    let num_lms_chars = buckets::place_text_order_lms_indices_into_buckets(
        suffix_array_buffer,
        bucket_indices_working_buffer,
        &is_s_type,
        text,
    );

    inducing::induce_to_sort_lms_substrings(
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

    buckets::place_sorted_lms_indices_into_buckets(
        suffix_array_buffer,
        num_lms_chars,
        bucket_start_indices,
        bucket_indices_working_buffer,
        text,
    );

    inducing::induce_to_finalize_suffix_array(
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

        // This transformation allows to only use a buffer of half the text size when placing the lexicographical
        // names into a buffer. It is sound because there cannot be LMS chars are directly neighboring positions
        // of the text and therefore, shifting to the right by one cannot lead to a collision of indices.
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
