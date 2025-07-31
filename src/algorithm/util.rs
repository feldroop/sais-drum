pub fn split_off_front_and_back_mut<T>(
    slice: &mut [T],
    front_offset: usize,
    back_offset: usize,
) -> (&mut [T], &mut [T], &mut [T]) {
    let (front, rest) = slice.split_at_mut(front_offset);
    let (mid, back) = rest.split_at_mut(rest.len() - back_offset);
    (front, mid, back)
}

pub fn split_off_same_front_and_back_mut<T>(
    slice: &mut [T],
    offset: usize,
) -> (&mut [T], &mut [T], &mut [T]) {
    split_off_front_and_back_mut(slice, offset, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_off_functions() {
        let mut array = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let (one, two, three) = split_off_front_and_back_mut(&mut array, 2, 3);
        assert_eq!(one, [1, 2]);
        assert_eq!(two, [3, 4, 5, 6]);
        assert_eq!(three, [7, 8, 9]);

        let (one, two, three) = split_off_same_front_and_back_mut(&mut array, 4);
        assert_eq!(one, [1, 2, 3, 4]);
        assert_eq!(two, [5]);
        assert_eq!(three, [6, 7, 8, 9]);
    }
}
