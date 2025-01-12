// #[cfg(test)]
// mod tests;

use crate::Character;

use bitvec::prelude::*;

use std::cmp::{self, Ordering};

// the text must always be smaller than this value
pub(crate) const NONE_VALUE: usize = usize::MAX;

// expects suffix array buffer to be filled with EMPTY_VALUE and of the same length as text
pub fn suffix_array_induced_sort<C: Character>(
    text: &[C],
    max_char: C,
    suffix_array_buffer: &mut [usize],
) {
    if text.is_empty() {
        return;
    }

    // TODO skip this scan in recursion
    let text_metadata = scan_for_counts_and_s_l_types(text, max_char);

    let num_lms_chars =
        place_text_order_lms_indices_into_buckets(suffix_array_buffer, &text_metadata, text);

    induce_to_sort_lms_substrings(
        suffix_array_buffer,
        &text_metadata.char_counts,
        &text_metadata.is_s_type,
        text,
    );

    let (reduced_text, vacant_buffer) = create_reduced_text(
        num_lms_chars,
        suffix_array_buffer,
        &text_metadata.is_s_type,
        text,
    );

    let mut reduced_text_suffix_array = vec![NONE_VALUE; reduced_text.data.len()];

    if reduced_text.num_different_names == num_lms_chars {
        // base case of recursion. this works, because the reduced text exclusively contains unique characters
        for (reduced_text_suffix_index, &reduced_text_char) in reduced_text.data.iter().enumerate()
        {
            reduced_text_suffix_array[reduced_text_char] = reduced_text_suffix_index;
        }
    } else {
        suffix_array_induced_sort(
            reduced_text.data,
            reduced_text.num_different_names - 1,
            &mut reduced_text_suffix_array,
        );
    };

    let original_text_sorted_lms_indices = backtransform_into_original_text_lms_indices(
        reduced_text_suffix_array,
        reduced_text.backtransformation_table,
    );

    // reinitialize buffer for inducing
    suffix_array_buffer.fill(NONE_VALUE);

    place_sorted_lms_indices_into_buckets(
        suffix_array_buffer,
        &original_text_sorted_lms_indices,
        &text_metadata.char_counts,
        text,
    );

    induce_to_finalize_suffix_array(
        suffix_array_buffer,
        &text_metadata.char_counts,
        &text_metadata.is_s_type,
        text,
    );
}

struct TextMetadata {
    is_s_type: BitVec,
    char_counts: Vec<usize>,
}

fn scan_for_counts_and_s_l_types<C: Character>(text: &[C], max_char: C) -> TextMetadata {
    let num_different_characters = max_char.rank() + 1;
    let mut char_counts = vec![0; num_different_characters];
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

    TextMetadata {
        is_s_type,
        char_counts,
    }
}

// returns number of lms chars, without sentinel
fn place_text_order_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    text_metadata: &TextMetadata,
    text: &[C],
) -> usize {
    let mut bucket_end_indices = bucket_end_indices_from_counts(&text_metadata.char_counts);
    let mut num_lms_chars = 0;
    for (text_index, char) in text.iter().enumerate().skip(1) {
        if !is_lms_type(text_index, &text_metadata.is_s_type) {
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

// suffix_array_buffer should be initialized with all MAX
fn place_sorted_lms_indices_into_buckets<C: Character>(
    suffix_array_buffer: &mut [usize],
    lms_indices: &[usize],
    char_counts: &[usize],
    text: &[C],
) {
    let mut bucket_end_indices = bucket_end_indices_from_counts(char_counts);

    for &lms_char_index in lms_indices.iter().rev() {
        let lms_char = text[lms_char_index];
        let bucket_end_index = &mut bucket_end_indices[lms_char.rank()];

        suffix_array_buffer[*bucket_end_index] = lms_char_index;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *bucket_end_index = bucket_end_index.saturating_sub(1);
    }
}

// after this, the sorted LMS indices (by LMS substrings) are at the end of suffix_array_buffer
fn induce_to_sort_lms_substrings<C: Character>(
    suffix_array_buffer: &mut [usize],
    char_counts: &[usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    // ---------- LEFT TO RIGHT SCAN ----------
    let mut bucket_start_indices = bucket_start_indices_from_counts(char_counts);

    // virtual sentinel induction, it would normally be at first position of the suffix array
    let last_suffix_index = text.len() - 1;
    let last_suffix_char = text[last_suffix_index];
    let last_suffix_bucket_start_index = &mut bucket_start_indices[last_suffix_char.rank()];
    suffix_array_buffer[*last_suffix_bucket_start_index] = last_suffix_index;
    *last_suffix_bucket_start_index += 1;

    for suffix_array_index in 0..suffix_array_buffer.len() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == NONE_VALUE || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_start_index =
            &mut bucket_start_indices[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_start_index] = suffix_index - 1;
        *induced_suffix_bucket_start_index += 1;
    }

    // ---------- RIGHT TO LEFT SCAN ----------

    let mut write_index = suffix_array_buffer.len() - 1;
    let mut bucket_end_indices = bucket_end_indices_from_counts(char_counts);
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
            &mut bucket_end_indices[induced_suffix_first_char.rank()];

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
    char_counts: &[usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    // ---------- LEFT TO RIGHT SCAN ----------
    let mut bucket_start_indices = bucket_start_indices_from_counts(char_counts);

    // virtual sentinel induction, it would normally be at first position of the suffix array
    let last_suffix_index = text.len() - 1;
    let last_suffix_char = text[last_suffix_index];
    let last_suffix_bucket_start_index = &mut bucket_start_indices[last_suffix_char.rank()];
    suffix_array_buffer[*last_suffix_bucket_start_index] = last_suffix_index;
    *last_suffix_bucket_start_index += 1;

    for suffix_array_index in 0..suffix_array_buffer.len() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == NONE_VALUE || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_start_index =
            &mut bucket_start_indices[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_start_index] = suffix_index - 1;
        *induced_suffix_bucket_start_index += 1;
    }

    // ---------- RIGHT TO LEFT SCAN ----------

    let mut bucket_end_indices = bucket_end_indices_from_counts(char_counts);
    for suffix_array_index in (0..suffix_array_buffer.len()).rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        // no need to check for EMPTY_VALUE here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 || !is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_end_index =
            &mut bucket_end_indices[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_end_index] = suffix_index - 1;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *induced_suffix_bucket_end_index = induced_suffix_bucket_end_index.saturating_sub(1);
    }

    // on the right to left scan, the sentinel does not induce anything,
    // because the char before it is always L-type
}

struct ReducedText<'a> {
    data: &'a [usize],
    num_different_names: usize,
    backtransformation_table: &'a [usize],
}

// reduced text is written to the beginning of the suffix array buffer
// expects suffix indices in suffix_array_buffer and LMS indices should be sorted according to
// their LMS substrings (not necessarily according to their whole LMS suffixes)
fn create_reduced_text<'a, C: Character>(
    num_lms_chars: usize,
    suffix_array_buffer: &'a mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) -> (ReducedText<'a>, &'a mut [usize]) {
    if num_lms_chars == 0 {
        return (
            ReducedText {
                data: &[],
                num_different_names: 0,
                backtransformation_table: &[],
            },
            suffix_array_buffer,
        );
    }

    let (reduced_text_placement_buffer, rest_of_suffix_array_buffer) =
        suffix_array_buffer.split_at_mut(text.len() / 2);

    reduced_text_placement_buffer.fill(NONE_VALUE);

    let start_of_lms_substring_indices = rest_of_suffix_array_buffer.len() - num_lms_chars;
    let sorted_lms_substring_indices =
        &mut rest_of_suffix_array_buffer[start_of_lms_substring_indices..];

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

    // reuse lms substring buffer as backtransformation_table
    let backtransformation_table = sorted_lms_substring_indices;

    let mut write_index = 0;
    for read_index in 0..reduced_text_placement_buffer.len() {
        let maybe_lms_substring_name = reduced_text_placement_buffer[read_index];

        if maybe_lms_substring_name != NONE_VALUE {
            reduced_text_placement_buffer[write_index] = maybe_lms_substring_name;

            // we need to undo the little shifting trick from above. there are two possiblities of where this entry
            // could come from, so we have to test which one of them is the actual lms char.
            // to avoid underflow for the index 0, we test the larger one.
            let index_of_original_text = if is_lms_type((read_index << 1) + 1, is_s_type) {
                (read_index << 1) + 1
            } else {
                read_index << 1
            };

            backtransformation_table[write_index] = index_of_original_text;
            write_index += 1;
        }
    }

    let (reduced_text_data, rest) = suffix_array_buffer.split_at_mut(num_lms_chars);
    let (vacant_buffer, backtransformation_table) = rest.split_at_mut(rest.len() - num_lms_chars);

    (
        ReducedText {
            data: reduced_text_data,
            num_different_names: current_name + 1,
            backtransformation_table,
        },
        vacant_buffer,
    )
}

fn backtransform_into_original_text_lms_indices(
    mut reduced_text_suffix_array: Vec<usize>,
    backtransformation_table: &[usize],
) -> Vec<usize> {
    for reduced_text_index in &mut reduced_text_suffix_array {
        *reduced_text_index = backtransformation_table[*reduced_text_index];
    }

    reduced_text_suffix_array // reuse buffer
}

// assumes index > 0
#[inline]
fn is_lms_type(index: usize, is_s_type: &BitSlice) -> bool {
    is_s_type[index] && !is_s_type[index - 1]
}

// inclusive index, the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
fn bucket_start_indices_from_counts(char_counts: &[usize]) -> Vec<usize> {
    char_counts
        .iter()
        .scan(0, |state, count| {
            let index = *state;
            *state += count;
            Some(index)
        })
        .collect()
}

// inclusive index, the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
fn bucket_end_indices_from_counts(char_counts: &[usize]) -> Vec<usize> {
    char_counts
        .iter()
        .scan(0, |state, count| {
            *state += count;
            Some(state.saturating_sub(1))
        })
        .collect()
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
