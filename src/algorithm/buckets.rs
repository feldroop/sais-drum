use std::{cmp, iter};

// inclusive index, the virtual bucket of the sentinel (count 1, ends at 0) is NOT included
pub fn counts_into_bucket_start_indices(char_counts: &mut [usize]) {
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
pub fn write_bucket_end_indices_into_buffer(
    bucket_start_indices: &[usize],
    bucket_indices_buffer: &mut [usize],
    text_len: usize,
) {
    for (bucket_end_index, bucket_buffer_position) in
        iter_bucket_end_indices(bucket_start_indices, text_len).zip(bucket_indices_buffer)
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

pub fn choose_larger_vacant_buffer<'a>(
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
