mod algorithm;

use num_traits::PrimInt;

use algorithm::NONE_VALUE;

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
        assert!(max_char.rank() < NONE_VALUE);
        self.max_char = Some(max_char);
        self
    }

    pub fn construct_suffix_array_inplace(&self, text: &[C], suffix_array_buffer: &mut [usize]) {
        assert_eq!(text.len(), suffix_array_buffer.len());
        // TODO maybe I can do this filling later, more efficiently
        suffix_array_buffer.fill(NONE_VALUE);

        algorithm::suffix_array_induced_sort(text, self.get_max_char(), suffix_array_buffer);
    }

    pub fn construct_suffix_array(&self, text: &[C]) -> Vec<usize> {
        let mut suffix_array_buffer = vec![algorithm::NONE_VALUE; text.len()];
        algorithm::suffix_array_induced_sort(text, self.get_max_char(), &mut suffix_array_buffer);

        suffix_array_buffer
    }

    fn get_max_char(&self) -> C {
        let max_char = self.max_char.unwrap_or(C::max_char());

        if max_char.rank() > u16::MAX as usize {
            todo!("for large alphabets, create a threshold where the text is scanned for max_char");
        }

        max_char
    }
}

impl<C: Character> Default for SaisBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}
