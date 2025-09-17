use crate::{Character, IndexStorage};

use std::cmp::Ordering;

use bitvec::slice::BitSlice;

// TODO: use first char rank
// TODO: 3 levels of bucket detail: just chars, chars + lms chars, chars + lms chars + 2-mers

pub struct TextMetadata<'a, I: IndexStorage> {
    pub is_s_type: &'a mut BitSlice<I>,
    pub first_char_rank: usize,
}

impl<'a, I: IndexStorage> TextMetadata<'a, I> {
    // assumes index > 0
    #[inline]
    pub fn is_lms_type(&self, index: I) -> bool {
        self.is_s_type[index.as_()] && !self.is_s_type[index.as_() - 1]
    }

    pub fn into_parts(self) -> (&'a mut BitSlice<I>, usize) {
        (self.is_s_type, self.first_char_rank)
    }

    pub fn from_filled_buffer_and_parts(
        is_s_type_buffer: &'a mut [I],
        first_char_rank: usize,
    ) -> Self {
        Self {
            is_s_type: BitSlice::from_slice_mut(is_s_type_buffer),
            first_char_rank,
        }
    }
}

pub fn scan_for_counts_and_s_l_types<'a, C: Character, I: IndexStorage>(
    text: &[C],
    persistent_bucket_start_indices_buffer: &mut [I],
    is_s_type_buffer: &'a mut [I],
) -> TextMetadata<'a, I> {
    let is_s_type = BitSlice::from_slice_mut(is_s_type_buffer);

    // sentinel is by definiton S-type and the smallest character
    let mut current_char_compared_to_previous = Ordering::Greater;
    is_s_type.set(text.len(), true);

    for (text_index, char) in text.iter().enumerate().rev() {
        let entry = &mut persistent_bucket_start_indices_buffer[char.rank()];
        *entry = *entry + I::one();

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

    let first_char_rank = text[0].rank();

    TextMetadata {
        is_s_type,
        first_char_rank,
    }
}
