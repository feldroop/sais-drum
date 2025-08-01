// #[cfg(test)]
// mod tests;

mod buckets;
pub mod buffer_management;
mod inducing;
mod text_analysis;
mod util;

use crate::Character;
use buffer_management::{BufferConfig, BufferRequestMode, BufferStack, Buffers};

use std::cmp;

use text_analysis::TextMetadata;

// the text must always be smaller than this value
pub(crate) const NONE_VALUE: usize = usize::MAX;
const WRAPPING_ZERO_DECREMENT_RESULT: usize = usize::MAX;

// TODO SaisConfig for configuration of this algorithm that is not the public API

// enum AlphabetSize {
//     Small,
//     Medium,
//     Large,
// }

// impl AlphabetSize {
//     fn from_num_chars(num_chars: usize) -> Self {
//         // TODO benchmark and optimize good values for these constants
//         if num_chars <= 256 {
//             Self::Small
//         } else if num_chars <= 256 * 256 {
//             Self::Medium
//         } else {
//             Self::Large
//         }
//     }
// }

// expects the main buffer to be of at least the same length as text
// and the values at 0..text.len() of main_buffer to be NONE_VALUE
pub fn suffix_array_induced_sort<C: Character>(
    text: &[C],
    max_char: C,
    main_buffer: &mut [usize],
    extra_buffers: &mut BufferStack,
) {
    if text.is_empty() {
        return;
    }

    let num_buckets = max_char.rank() + 1;
    let buffer_config = BufferConfig::calculate(text.len(), main_buffer.len(), num_buckets);

    let Buffers {
        remaining_main_buffer_without_persistent_buffers,
        // persistent means the buffer is kept during recursion, while the working buffer is reused
        // the s-type buffer is also persistent
        is_s_type_buffer,
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
    } = buffer_management::instantiate_or_recover_buffers(
        buffer_config,
        main_buffer,
        extra_buffers,
        num_buckets,
        BufferRequestMode::Instatiate,
    );

    // if the working buffer is allocated in the surplus main buffer, the value returned is None.
    // Then, this split needs to happen in this function to allow reobtaining the remaining main
    // buffer without persistent buffers later to use it fully in the recursion
    let (final_remaining_main_buffer, working_bucket_indices_buffer) =
        if buffer_config.working_bucket_buffer_in_main_buffer {
            remaining_main_buffer_without_persistent_buffers
                .split_at_mut(remaining_main_buffer_without_persistent_buffers.len() - num_buckets)
        } else {
            (
                &mut remaining_main_buffer_without_persistent_buffers[..],
                working_bucket_indices_buffer.unwrap(),
            )
        };

    let suffix_array_buffer = &mut final_remaining_main_buffer[..text.len()];

    // TODO maybe skip this scan in recursion
    let text_metadata = text_analysis::scan_for_counts_and_s_l_types(
        text,
        persistent_bucket_start_indices_buffer,
        is_s_type_buffer,
    );
    buckets::counts_into_bucket_start_indices(persistent_bucket_start_indices_buffer);

    let num_lms_chars = buckets::place_text_order_lms_indices_into_buckets(
        suffix_array_buffer,
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
        text,
        &text_metadata,
    );

    inducing::induce_to_sort_lms_substrings(
        suffix_array_buffer,
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
        &text_metadata,
        text,
    );

    let (front, _, lms_indices) =
        util::split_off_same_front_and_back_mut(suffix_array_buffer, num_lms_chars);
    // TODO maybe in the future this will be a step that gathers the scattered lms indices to the front
    front.copy_from_slice(lms_indices);

    let num_different_names = create_reduced_text(
        num_lms_chars,
        remaining_main_buffer_without_persistent_buffers,
        &text_metadata,
        text,
    );

    // text_metadata needs to be destructed, because it might borrow an extra buffer
    let (_, first_char_rank) = text_metadata.into_parts();

    let (main_buffer_for_recursion, reduced_text) =
        remaining_main_buffer_without_persistent_buffers
            .split_at_mut(remaining_main_buffer_without_persistent_buffers.len() - num_lms_chars);

    buffer_management::setup_for_recursion(buffer_config, extra_buffers);

    if num_different_names == num_lms_chars {
        directly_construct_suffix_array(reduced_text, main_buffer_for_recursion);
    } else {
        main_buffer_for_recursion[..num_lms_chars].fill(NONE_VALUE);

        suffix_array_induced_sort(
            reduced_text,
            num_different_names - 1,
            main_buffer_for_recursion,
            extra_buffers,
        );
    };

    // here the whole buffer structure needs to be setup again to make sure everything
    // except the persistent buffers can be mutably borrowoed in the recursion
    let Buffers {
        remaining_main_buffer_without_persistent_buffers,
        is_s_type_buffer,
        // persistent means it is kept during recursion
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
    } = buffer_management::instantiate_or_recover_buffers(
        buffer_config,
        main_buffer,
        extra_buffers,
        num_buckets,
        BufferRequestMode::Recover,
    );

    let text_metadata =
        TextMetadata::from_filled_buffer_and_parts(is_s_type_buffer, first_char_rank);

    // reuse reduced text buffer for backtransformation table
    let (main_buffer_for_recursion, backtransformation_table) =
        remaining_main_buffer_without_persistent_buffers
            .split_at_mut(remaining_main_buffer_without_persistent_buffers.len() - num_lms_chars);

    create_backtransformation_table(backtransformation_table, &text_metadata, text.len());

    backtransform_into_original_text_lms_indices(
        &mut main_buffer_for_recursion[..num_lms_chars],
        backtransformation_table,
    );

    // setup working bucket indices buffer again, just like above
    let (final_remaining_main_buffer, working_bucket_indices_buffer) =
        if buffer_config.working_bucket_buffer_in_main_buffer {
            remaining_main_buffer_without_persistent_buffers
                .split_at_mut(remaining_main_buffer_without_persistent_buffers.len() - num_buckets)
        } else {
            (
                remaining_main_buffer_without_persistent_buffers,
                working_bucket_indices_buffer.unwrap(),
            )
        };

    let suffix_array_buffer = &mut final_remaining_main_buffer[..text.len()];

    buckets::place_sorted_lms_indices_into_buckets(
        suffix_array_buffer,
        num_lms_chars,
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
        text,
    );

    inducing::induce_to_finalize_suffix_array(
        suffix_array_buffer,
        persistent_bucket_start_indices_buffer,
        working_bucket_indices_buffer,
        &text_metadata,
        text,
    );

    buffer_management::clean_up_extra_buffers(buffer_config, extra_buffers);
}

// reduced text is written to the end of the suffix array buffer
// LMS indices at front of input suffix_array_buffer should be sorted according to
// their LMS substrings (not necessarily according to their whole LMS suffixes)
// returns number of different names in the text
fn create_reduced_text<C: Character>(
    num_lms_chars: usize,
    remaining_main_buffer_without_persistent_buffers: &mut [usize],
    text_metadata: &TextMetadata,
    text: &[C],
) -> usize {
    if num_lms_chars == 0 {
        return 0;
    }

    let (sorted_lms_substring_indices, _, reduced_text_placement_buffer) =
        util::split_off_front_and_back_mut(
            remaining_main_buffer_without_persistent_buffers,
            num_lms_chars,
            text.len().div_ceil(2),
        );
    reduced_text_placement_buffer.fill(NONE_VALUE);

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
            text_metadata,
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

fn create_backtransformation_table(
    backtransformation_table: &mut [usize],
    text_metadata: &TextMetadata,
    text_len: usize,
) {
    let mut write_index = 0;

    for text_index in 1..text_len - 1 {
        if text_metadata.is_lms_type(text_index) {
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

// this assumes, that the two given indices are different. otherwise it might return true
// in some edge cases, when first_lms_substring_index == second_lms_substring_index
fn lms_substrings_are_unequal<C: Character>(
    first_lms_substring_index: usize,
    second_lms_substring_index: usize,
    text_metadata: &TextMetadata,
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

        let first_substring_is_over = text_metadata.is_lms_type(smaller_lms_substring_index);
        let second_substring_is_over = text_metadata.is_lms_type(larger_lms_substring_index);

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
