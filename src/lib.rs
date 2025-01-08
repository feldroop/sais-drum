use bitvec::prelude::*;
use num_traits::PrimInt;

use std::cmp::{self, Ordering};

pub trait Character: Sized + Copy + Ord {
    fn max_char() -> Self;

    fn rank(&self) -> usize;
}

impl<P: PrimInt> Character for P {
    fn max_char() -> Self {
        P::max_value()
    }

    fn rank(&self) -> usize {
        self.to_usize().unwrap()
    }
}

pub struct SaisBuilder<C> {
    max_char: Option<C>,
}

impl<C: Character> SaisBuilder<C> {
    pub fn new() -> Self {
        Self { max_char: None }
    }

    // if I ever remove bounds checks, this would become unsafe (then add checks and an unchecked method)
    pub fn with_max_char(&mut self, max_char: C) -> &mut Self {
        assert!(max_char.rank() < usize::MAX);
        self.max_char = Some(max_char);
        self
    }

    pub fn construct_suffix_array_inplace(&self, text: &[C], suffix_array_buffer: &mut [usize]) {
        assert_eq!(text.len(), suffix_array_buffer.len());
        suffix_array_buffer.fill(usize::MAX);

        suffix_array_induced_sort(text, self.get_max_char(), suffix_array_buffer);
    }

    pub fn construct_suffix_array(&self, text: &[C]) -> Vec<usize> {
        let mut suffix_array_buffer = vec![usize::MAX; text.len()];
        suffix_array_induced_sort(text, self.get_max_char(), &mut suffix_array_buffer);

        suffix_array_buffer
    }

    fn get_max_char(&self) -> C {
        self.max_char.unwrap_or(C::max_char())
    }
}

impl<C: Character> Default for SaisBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

// expects suffix array buffer to be filled with usize::MAX and of the same length as text
fn suffix_array_induced_sort<C: Character>(
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

    // lms char indices currently in REVERSE text order, first one is virtual sentinel
    for lms_char_index in lms_indices.into_iter() {
        let lms_char = text[lms_char_index];
        let bucket_end_index = &mut bucket_end_indices[lms_char.rank()];

        suffix_array_buffer[*bucket_end_index] = lms_char_index;
        *bucket_end_index -= 1;
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
    let mut bucket_start_indices_for_induction = bucket_start_indices_from_counts(char_counts);

    // virtual sentinel induction, it would normally be at first position of the suffix array
    let last_suffix_index = text.len() - 1;
    let last_suffix_char = text[last_suffix_index];
    let last_suffix_bucket_start_index =
        &mut bucket_start_indices_for_induction[last_suffix_char.rank()];
    suffix_array_buffer[*last_suffix_bucket_start_index] = last_suffix_index;
    *last_suffix_bucket_start_index += 1;

    for suffix_array_index in 0..suffix_array_buffer.len() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        if suffix_index == usize::MAX || suffix_index == 0 || is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_start_index =
            &mut bucket_start_indices_for_induction[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_start_index] = suffix_index - 1;
        *induced_suffix_bucket_start_index += 1;
    }

    // ---------- RIGHT TO LEFT SCAN ----------

    let mut bucket_end_indices_for_induction = bucket_end_indices_from_counts(char_counts);
    for suffix_array_index in (0..suffix_array_buffer.len()).rev() {
        let suffix_index = suffix_array_buffer[suffix_array_index];

        // no need to check for usize::MAX here, because in this iteration, every index of the suffix
        // array buffer must have been written to before (L-type in previous scan, S-type in this one)
        if suffix_index == 0 || !is_s_type[suffix_index - 1] {
            continue;
        }

        let induced_suffix_first_char = text[suffix_index - 1];
        let induced_suffix_bucket_end_index =
            &mut bucket_end_indices_for_induction[induced_suffix_first_char.rank()];

        suffix_array_buffer[*induced_suffix_bucket_end_index] = suffix_index - 1;
        *induced_suffix_bucket_end_index -= 1;
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

// the virtual bucket of the sentinel (count 1, starts at 0) is NOT included
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

#[cfg(test)]
mod tests {
    use std::{iter, sync::LazyLock};

    use super::*;

    // example from https://ae.iti.kit.edu/download/kurpicz/2022_text_indexing/02_suffix_tree_and_array_handout_ws2223.pdf
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
        LazyLock::new(|| scan_for_counts_types_and_lms_chars(ABC_TEXT, 255));
    static ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES: &[usize] = &[8, 2, 5];

    #[test]
    fn test_scan_for_counts_types_and_lms_chars_u8_abc_text() {
        assert_eq!(
            ABC_TEXT_METADATA.is_s_type,
            bitvec::bits![1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1]
        );

        assert_eq!(
            ABC_TEXT_METADATA.reverse_order_lms_char_indices,
            [12, 8, 5, 2]
        );
        assert_eq!(&ABC_TEXT_METADATA.char_counts, &*ABC_TEXT_EXPECTED_COUNTS);
    }

    #[test]
    fn test_bucket_indices_from_counts_u8_abc_text() {
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
    fn test_lms_substring_sorting_u8_abc_text() {
        let mut suffix_array_buffer = vec![usize::MAX; ABC_TEXT.len()];

        initialize_lms_indices_and_induce(
            &mut suffix_array_buffer,
            ABC_TEXT_METADATA
                .reverse_order_lms_char_indices
                .iter()
                .skip(1)
                .copied(),
            &ABC_TEXT_METADATA.char_counts,
            &ABC_TEXT_METADATA.is_s_type,
            ABC_TEXT,
        );

        // different than in the example slides, because multiple results are possible
        // it would be better to check that all LMS substrings are sorted instead of a fixed result
        assert_eq!(
            suffix_array_buffer,
            vec![11, 0, 8, 2, 5, 10, 1, 9, 3, 6, 4, 7]
        );

        let sorted_lms_substring_indices = extract_sorted_lms_substring_indices(
            &ABC_TEXT_METADATA.is_s_type,
            &suffix_array_buffer,
        );

        assert_eq!(
            sorted_lms_substring_indices,
            ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES
        );
    }

    #[test]
    fn test_create_reduced_text_u8_abc_text() {
        let mut suffix_array_buffer = vec![usize::MAX; ABC_TEXT.len()];

        let reduced_text_metadata = create_reduced_text(
            ABC_TEXT_LEX_SORTED_LMS_SUBSTRING_INDICES,
            &mut suffix_array_buffer,
            &ABC_TEXT_METADATA.is_s_type,
            ABC_TEXT,
        );

        assert_eq!(reduced_text_metadata.data.len(), 3);
        assert_eq!(reduced_text_metadata.num_different_names, 2);
        assert_eq!(reduced_text_metadata.backtransformation_table, &[2, 5, 8]);
        assert_eq!(reduced_text_metadata.data, [1, 1, 0]); // mistake in the example slides
    }

    #[test]
    fn test_lms_substrings_are_unequal_u8_abc_text() {
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

    #[test]
    fn test_whole_algorithm_u8_abc_text() {
        let naive_result = construct_suffix_array_naive(ABC_TEXT);
        let result = SaisBuilder::new().construct_suffix_array(ABC_TEXT);

        assert_eq!(naive_result, [11, 0, 8, 5, 2, 10, 1, 9, 6, 3, 7, 4]);
        assert_eq!(result, naive_result);
    }

    #[test]
    fn test_whole_algorithm_short_texts() {
        let empty_text: [u8; 0] = [];
        let result_zero = SaisBuilder::new().construct_suffix_array(&empty_text);
        let result_one = SaisBuilder::new().construct_suffix_array(&[42u8]);
        let result_two = SaisBuilder::new()
            .with_max_char(42usize)
            .construct_suffix_array(&[42usize, 3]);

        assert_eq!(result_zero, []);
        assert_eq!(result_one, [0]);
        assert_eq!(result_two, [1, 0]);
    }

    fn construct_suffix_array_naive(text: &[u8]) -> Vec<usize> {
        let mut suffix_array: Vec<_> = (0..text.len()).collect();
        suffix_array.sort_unstable_by_key(|&index| &text[index..]);
        suffix_array
    }
}
