#[cfg(test)]
mod tests;

use crate::Character;

use bitvec::prelude::*;

use std::cmp::{self, Ordering};

// expects suffix array buffer to be filled with usize::MAX and of the same length as text
pub fn suffix_array_induced_sort<C: Character>(
    text: &[C],
    max_char: C,
    suffix_array_buffer: &mut [usize],
) {
    if text.is_empty() {
        return;
    }

    let text_metadata = scan_for_counts_types_and_lms_chars(text, max_char);

    // sort LMS substrings
    initialize_lms_indices_and_induce(
        suffix_array_buffer,
        text_metadata
            .reverse_order_lms_char_indices
            .iter()
            .skip(1) // skip sentinel, because it is handles in a special case
            .copied(),
        &text_metadata.char_counts,
        &text_metadata.is_s_type,
        text,
    );

    let sorted_lms_substring_indices =
        extract_sorted_lms_substring_indices(&text_metadata.is_s_type, suffix_array_buffer);

    let reduced_text = create_reduced_text(
        &sorted_lms_substring_indices,
        suffix_array_buffer,
        &text_metadata.is_s_type,
        text,
    );

    let mut reduced_text_suffix_array = vec![usize::MAX; reduced_text.data.len()];

    if reduced_text.num_different_names == sorted_lms_substring_indices.len() {
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
        &reduced_text.backtransformation_table,
    );

    // reinitialize buffer for inducing
    suffix_array_buffer.fill(usize::MAX);

    initialize_lms_indices_and_induce(
        suffix_array_buffer,
        original_text_sorted_lms_indices.iter().rev().copied(),
        &text_metadata.char_counts,
        &text_metadata.is_s_type,
        text,
    );
}

struct TextMetadata {
    is_s_type: BitVec,
    reverse_order_lms_char_indices: Vec<usize>,
    char_counts: Vec<usize>,
}

fn scan_for_counts_types_and_lms_chars<C: Character>(text: &[C], max_char: C) -> TextMetadata {
    let num_different_characters = max_char.rank() + 1;
    let mut char_counts = vec![0; num_different_characters];

    // t[] in paper, virtual sentinel at the end
    let mut is_s_type = BitVec::repeat(true, text.len() + 1);

    // P_1 in paper
    let mut lms_char_indices = Vec::new();

    // sentinel is by definiton S-type and the smallest character
    let mut previous_char_is_s_type = true;
    let mut current_char_compared_to_previous = Ordering::Greater;

    for current_index in (0..text.len()).rev() {
        char_counts[text[current_index].rank()] += 1;

        let current_char_is_s_type = match current_char_compared_to_previous {
            Ordering::Less => true,
            Ordering::Equal => is_s_type[current_index + 1],
            Ordering::Greater => false,
        };

        is_s_type.set(current_index, current_char_is_s_type);

        if !current_char_is_s_type && previous_char_is_s_type {
            lms_char_indices.push(current_index + 1);
        }

        if current_index == 0 {
            break;
        }

        previous_char_is_s_type = current_char_is_s_type;
        current_char_compared_to_previous = text[current_index - 1].cmp(&text[current_index])
    }

    TextMetadata {
        is_s_type,
        reverse_order_lms_char_indices: lms_char_indices,
        char_counts,
    }
}

// suffix_array_buffer should be initialized with all MAX
fn initialize_lms_indices_and_induce<C: Character>(
    suffix_array_buffer: &mut [usize],
    lms_indices: impl IntoIterator<Item = usize>,
    char_counts: &[usize],
    is_s_type: &BitSlice,
    text: &[C],
) {
    let mut bucket_end_indices = bucket_end_indices_from_counts(char_counts);

    for lms_char_index in lms_indices.into_iter() {
        let lms_char = text[lms_char_index];
        let bucket_end_index = &mut bucket_end_indices[lms_char.rank()];

        suffix_array_buffer[*bucket_end_index] = lms_char_index;

        // saturating sub used, because the last placement of the first bucket (index 0) otherwise might underflow
        // (it is okay to keep zero, because it is never read again. might also just use underflowing function)
        *bucket_end_index = bucket_end_index.saturating_sub(1);
    }

    induce(suffix_array_buffer, char_counts, is_s_type, text);
}

fn induce<C: Character>(
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

        if suffix_index == usize::MAX || suffix_index == 0 || is_s_type[suffix_index - 1] {
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

        // no need to check for usize::MAX here, because in this iteration, every index of the suffix
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

fn extract_sorted_lms_substring_indices(
    is_s_type: &BitSlice,
    suffix_array_buffer: &[usize],
) -> Vec<usize> {
    let mut lms_substring_indices = Vec::new();

    for &suffix_index in suffix_array_buffer {
        if suffix_index != 0 && is_lms_type(suffix_index, is_s_type) {
            lms_substring_indices.push(suffix_index);
        }
    }

    lms_substring_indices
}

struct ReducedText<'a> {
    data: &'a [usize],
    num_different_names: usize,
    backtransformation_table: Vec<usize>,
}

// reduced text is written to the beginning of the suffix array buffer
fn create_reduced_text<'a, C: Character>(
    sorted_lms_substring_indices: &[usize],
    suffix_array_buffer: &'a mut [usize],
    is_s_type: &BitSlice,
    text: &[C],
) -> ReducedText<'a> {
    if sorted_lms_substring_indices.is_empty() {
        return ReducedText {
            data: &suffix_array_buffer[..0],
            num_different_names: 0,
            backtransformation_table: Vec::new(),
        };
    }

    suffix_array_buffer.fill(usize::MAX);
    let mut current_name = 0;

    for index_of_sorted_lms_substring_indices in 0..sorted_lms_substring_indices.len() - 1 {
        let curr_lms_substring_index =
            sorted_lms_substring_indices[index_of_sorted_lms_substring_indices];

        suffix_array_buffer[curr_lms_substring_index] = current_name;

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

    suffix_array_buffer[*sorted_lms_substring_indices.last().unwrap()] = current_name;

    // given the position in the reduced text, return corresponding LMS substring index of original text
    let mut backtransformation_table = vec![0; sorted_lms_substring_indices.len()];

    let mut write_index = 0;
    for read_index in 0..suffix_array_buffer.len() {
        let maybe_lms_substring_name = suffix_array_buffer[read_index];

        if maybe_lms_substring_name != usize::MAX {
            suffix_array_buffer[write_index] = maybe_lms_substring_name;
            backtransformation_table[write_index] = read_index;
            write_index += 1;
        }
    }

    ReducedText {
        data: &suffix_array_buffer[..write_index],
        num_different_names: current_name + 1,
        backtransformation_table,
    }
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

    if text[smaller_lms_substring_index] != text[larger_lms_substring_index]
        || text[smaller_lms_substring_index + 1] != text[larger_lms_substring_index + 1]
    {
        return true;
    }

    // this is in bounds, because every lms substring has at least length 3, except for the sentinel,
    // but it is a unique chracter and would be caught by the above if statement.
    // also, the sentinel is handled as a virtual character, so it does never appear in this function
    smaller_lms_substring_index += 2;
    larger_lms_substring_index += 2;

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
