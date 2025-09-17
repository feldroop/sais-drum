mod algorithm;

use std::marker::PhantomData;

use bitvec::store::BitStore;
use num::Integer;
use num_traits::{AsPrimitive, NumCast, PrimInt, SaturatingSub, WrappingSub};

use algorithm::buffer_management::BufferStack;

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

pub trait IndexStorage:
    PrimInt + BitStore + AsPrimitive<usize> + WrappingSub + SaturatingSub + Integer
{
}

impl IndexStorage for u8 {}
impl IndexStorage for u16 {}
impl IndexStorage for u32 {}
impl IndexStorage for u64 {}
impl IndexStorage for usize {}

pub struct SaisBuilder<C = u8, I = usize> {
    max_char: Option<C>,
    _marker: PhantomData<I>,
}

impl<C: Character, I: IndexStorage> SaisBuilder<C, I> {
    pub fn new() -> Self {
        Self {
            max_char: None,
            _marker: PhantomData,
        }
    }

    // if I ever remove bounds checks, this would become unsafe (then add checks and an unchecked method)
    pub fn with_max_char(&mut self, max_char: C) -> &mut Self {
        assert!(max_char.rank() < <usize as NumCast>::from(I::max_value()).unwrap());
        self.max_char = Some(max_char);
        self
    }

    pub fn construct_suffix_array_inplace(&self, text: &[C], suffix_array_buffer: &mut [I]) {
        assert!(text.len() <= suffix_array_buffer.len());
        suffix_array_buffer[..text.len()].fill(I::max_value());

        let mut extra_buffer = BufferStack::new();

        algorithm::suffix_array_induced_sort(
            text,
            self.get_max_char(),
            suffix_array_buffer,
            &mut extra_buffer,
        );
    }

    pub fn construct_suffix_array(&self, text: &[C]) -> Vec<I> {
        let mut suffix_array_buffer = vec![I::max_value(); text.len()];
        let mut extra_buffer = BufferStack::new();

        algorithm::suffix_array_induced_sort(
            text,
            self.get_max_char(),
            &mut suffix_array_buffer,
            &mut extra_buffer,
        );

        suffix_array_buffer
    }

    fn get_max_char(&self) -> C {
        let max_char = self.max_char.unwrap_or(C::max_char());

        if max_char.rank() > u16::MAX as usize {
            unimplemented!(
                "for large alphabets, create a threshold where the text is scanned for max_char"
            );
        }

        max_char
    }
}

impl<C: Character, I: IndexStorage> Default for SaisBuilder<C, I> {
    fn default() -> Self {
        Self::new()
    }
}
