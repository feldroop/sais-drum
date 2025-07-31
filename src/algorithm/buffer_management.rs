use super::util;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BufferRequestMode {
    Instatiate,
    Recover,
}

/// A stack of buffers, backed by a single large buffer
pub struct BufferStack {
    full_buffer: Vec<usize>,
    individual_buffer_lengths: Vec<usize>,
}

impl BufferStack {
    pub fn new() -> Self {
        Self {
            full_buffer: Vec::new(),
            individual_buffer_lengths: Vec::new(),
        }
    }

    fn push(&mut self, new_buffer_length: usize) -> &mut [usize] {
        let old_len = self.full_buffer.len();
        self.full_buffer.resize(old_len + new_buffer_length, 0);
        self.individual_buffer_lengths.push(new_buffer_length);

        &mut self.full_buffer[old_len..]
    }

    fn push_two(
        &mut self,
        new_buffer_length1: usize,
        new_buffer_length2: usize,
    ) -> [&mut [usize]; 2] {
        let old_len = self.full_buffer.len();
        let total_new_length = new_buffer_length1 + new_buffer_length2;
        self.full_buffer.resize(old_len + total_new_length, 0);

        self.individual_buffer_lengths.push(new_buffer_length1);
        self.individual_buffer_lengths.push(new_buffer_length2);

        self.full_buffer[old_len..]
            .split_at_mut(new_buffer_length1)
            .into()
    }

    fn push_three(
        &mut self,
        new_buffer_length1: usize,
        new_buffer_length2: usize,
        new_buffer_length3: usize,
    ) -> [&mut [usize]; 3] {
        let old_len = self.full_buffer.len();
        let total_new_length = new_buffer_length1 + new_buffer_length2 + new_buffer_length3;
        self.full_buffer.resize(old_len + total_new_length, 0);

        self.individual_buffer_lengths.push(new_buffer_length1);
        self.individual_buffer_lengths.push(new_buffer_length2);
        self.individual_buffer_lengths.push(new_buffer_length3);

        util::split_off_front_and_back_mut(
            &mut self.full_buffer[old_len..],
            new_buffer_length1,
            new_buffer_length3,
        )
        .into()
    }

    fn pop(&mut self) -> bool {
        let Some(last_buffer_length) = self.individual_buffer_lengths.pop() else {
            return false;
        };

        let old_len = self.full_buffer.len();
        self.full_buffer.truncate(old_len - last_buffer_length);

        true
    }

    fn peek(&mut self) -> &mut [usize] {
        assert!(self.individual_buffer_lengths.len() >= 1);

        let full_len = self.full_buffer.len();
        let last_buffer_length = self.individual_buffer_lengths.last().unwrap();

        &mut self.full_buffer[full_len - last_buffer_length..]
    }

    fn peek_two(&mut self) -> [&mut [usize]; 2] {
        let num_buffers = self.individual_buffer_lengths.len();
        assert!(num_buffers >= 2);

        let last_buffer_length = self.individual_buffer_lengths[num_buffers - 1];
        let second_last_buffer_length = self.individual_buffer_lengths[num_buffers - 2];

        let full_len = self.full_buffer.len();
        let (remaining, last_buffer) = self.full_buffer.split_at_mut(full_len - last_buffer_length);

        let remaining_len = remaining.len();
        let (_, second_last_buffer) =
            remaining.split_at_mut(remaining_len - second_last_buffer_length);

        [second_last_buffer, last_buffer]
    }

    fn peek_three(&mut self) -> [&mut [usize]; 3] {
        let num_buffers = self.individual_buffer_lengths.len();
        assert!(num_buffers >= 3);

        let last_buffer_length = self.individual_buffer_lengths[num_buffers - 1];
        let second_last_buffer_length = self.individual_buffer_lengths[num_buffers - 2];
        let third_last_buffer_length = self.individual_buffer_lengths[num_buffers - 3];

        let full_len = self.full_buffer.len();
        let (remaining, last_buffer) = self.full_buffer.split_at_mut(full_len - last_buffer_length);

        let remaining_len = remaining.len();
        let (remaining, second_last_buffer) =
            remaining.split_at_mut(remaining_len - second_last_buffer_length);

        let remaining_len = remaining.len();
        let (_, third_last_buffer) =
            remaining.split_at_mut(remaining_len - third_last_buffer_length);

        [third_last_buffer, second_last_buffer, last_buffer]
    }

    fn push_or_peek(
        &mut self,
        new_buffer_length: usize,
        buffer_request_mode: BufferRequestMode,
    ) -> &mut [usize] {
        match buffer_request_mode {
            BufferRequestMode::Instatiate => self.push(new_buffer_length),
            BufferRequestMode::Recover => self.peek(),
        }
    }

    fn push_or_peek_two(
        &mut self,
        new_buffer_length1: usize,
        new_buffer_length2: usize,
        buffer_request_mode: BufferRequestMode,
    ) -> [&mut [usize]; 2] {
        match buffer_request_mode {
            BufferRequestMode::Instatiate => self.push_two(new_buffer_length1, new_buffer_length2),
            BufferRequestMode::Recover => self.peek_two(),
        }
    }

    fn push_or_peek_three(
        &mut self,
        new_buffer_length1: usize,
        new_buffer_length2: usize,
        new_buffer_length3: usize,
        buffer_request_mode: BufferRequestMode,
    ) -> [&mut [usize]; 3] {
        match buffer_request_mode {
            BufferRequestMode::Instatiate => {
                self.push_three(new_buffer_length1, new_buffer_length2, new_buffer_length3)
            }
            BufferRequestMode::Recover => self.peek_three(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferConfig {
    pub is_s_type_buffer_size: usize,
    pub is_s_type_buffer_in_main_buffer: bool,
    pub persistent_bucket_buffer_in_main_buffer: bool,
    pub working_bucket_buffer_in_main_buffer: bool,
}

impl BufferConfig {
    pub fn calculate(text_len: usize, main_buffer_len: usize, num_buckets: usize) -> Self {
        let is_s_type_buffer_size = (text_len + 1).div_ceil(usize::BITS as usize);
        let is_s_type_buffer_is_larger = is_s_type_buffer_size > num_buckets;

        let mut buffer_config = BufferConfig {
            is_s_type_buffer_size,
            is_s_type_buffer_in_main_buffer: false,
            persistent_bucket_buffer_in_main_buffer: false,
            working_bucket_buffer_in_main_buffer: false,
        };

        let mut remaining_surplus_buffer_len = main_buffer_len - text_len;

        if is_s_type_buffer_is_larger
            && !(2 * num_buckets <= remaining_surplus_buffer_len
                && is_s_type_buffer_size + num_buckets > remaining_surplus_buffer_len)
        {
            if remaining_surplus_buffer_len >= is_s_type_buffer_size {
                buffer_config.is_s_type_buffer_in_main_buffer = true;
                remaining_surplus_buffer_len -= is_s_type_buffer_size;
            }

            if remaining_surplus_buffer_len >= num_buckets {
                buffer_config.persistent_bucket_buffer_in_main_buffer = true;
                remaining_surplus_buffer_len -= num_buckets;
            }

            if remaining_surplus_buffer_len >= num_buckets {
                buffer_config.working_bucket_buffer_in_main_buffer = true;
            }
        } else {
            if remaining_surplus_buffer_len >= num_buckets {
                buffer_config.persistent_bucket_buffer_in_main_buffer = true;
                remaining_surplus_buffer_len -= num_buckets;
            }

            if remaining_surplus_buffer_len >= num_buckets {
                buffer_config.working_bucket_buffer_in_main_buffer = true;
                remaining_surplus_buffer_len -= num_buckets;
            }

            if remaining_surplus_buffer_len >= is_s_type_buffer_size {
                buffer_config.is_s_type_buffer_in_main_buffer = true;
            }
        }

        buffer_config
    }

    pub fn num_extra_buffers(&self) -> usize {
        [
            self.is_s_type_buffer_in_main_buffer,
            self.persistent_bucket_buffer_in_main_buffer,
            self.working_bucket_buffer_in_main_buffer,
        ]
        .iter()
        .filter(|&&x| !x)
        .count()
    }
}

pub struct Buffers<'m, 'e> {
    pub remaining_main_buffer_without_persistent_buffers: &'m mut [usize],
    pub is_s_type_buffer: &'e mut [usize],
    pub persistent_bucket_start_indices_buffer: &'e mut [usize],
    pub working_bucket_indices_buffer: Option<&'e mut [usize]>,
}

// if buffer request mode is instatiate, then returned is_s_type_buffer and
// persistent_bucket_start_indices_buffer are guarenteed to be filled with zeroes
pub fn instantiate_or_recover_buffers<'e, 'm: 'e>(
    buffer_config: BufferConfig,
    main_buffer: &'m mut [usize],
    extra_buffers: &'e mut BufferStack,
    num_buckets: usize,
    buffer_request_mode: BufferRequestMode,
) -> Buffers<'m, 'e> {
    let mut remaining_main_buffer = main_buffer;
    let mut is_s_type_buffer = None;
    let mut persistent_bucket_start_indices_buffer = None;
    let mut working_bucket_indices_buffer = None;

    if buffer_config.is_s_type_buffer_in_main_buffer {
        let (remaining, buffer) = remaining_main_buffer
            .split_at_mut(remaining_main_buffer.len() - buffer_config.is_s_type_buffer_size);

        remaining_main_buffer = remaining;

        if buffer_request_mode == BufferRequestMode::Instatiate {
            buffer.fill(0);
        }
        is_s_type_buffer = Some(buffer);
    }

    if buffer_config.persistent_bucket_buffer_in_main_buffer {
        let (remaining, buffer) =
            remaining_main_buffer.split_at_mut(remaining_main_buffer.len() - num_buckets);

        remaining_main_buffer = remaining;

        if buffer_request_mode == BufferRequestMode::Instatiate {
            buffer.fill(0);
        }
        persistent_bucket_start_indices_buffer = Some(buffer);
    }

    match (
        buffer_config.is_s_type_buffer_in_main_buffer,
        buffer_config.persistent_bucket_buffer_in_main_buffer,
        buffer_config.working_bucket_buffer_in_main_buffer,
    ) {
        (true, true, true) => {}
        (true, true, false) => {
            working_bucket_indices_buffer =
                Some(extra_buffers.push_or_peek(num_buckets, buffer_request_mode))
        }
        (true, false, true) => panic!("Unexpected internal bug in buffer instantiation"),
        (true, false, false) => {
            [
                persistent_bucket_start_indices_buffer,
                working_bucket_indices_buffer,
            ] = extra_buffers
                .push_or_peek_two(num_buckets, num_buckets, buffer_request_mode)
                .map(|x| Some(x))
        }
        (false, true, true) => {
            is_s_type_buffer = Some(extra_buffers.push(buffer_config.is_s_type_buffer_size))
        }
        (false, true, false) => {
            [is_s_type_buffer, working_bucket_indices_buffer] = extra_buffers
                .push_two(buffer_config.is_s_type_buffer_size, num_buckets)
                .map(|x| Some(x))
        }
        (false, false, true) => panic!("Unexpected internal bug in buffer instantiation"),
        (false, false, false) => {
            [
                is_s_type_buffer,
                persistent_bucket_start_indices_buffer,
                working_bucket_indices_buffer,
            ] = extra_buffers
                .push_or_peek_three(
                    buffer_config.is_s_type_buffer_size,
                    num_buckets,
                    num_buckets,
                    buffer_request_mode,
                )
                .map(|x| Some(x))
        }
    }

    Buffers {
        remaining_main_buffer_without_persistent_buffers: remaining_main_buffer,
        is_s_type_buffer: is_s_type_buffer.unwrap(),
        persistent_bucket_start_indices_buffer: persistent_bucket_start_indices_buffer.unwrap(),
        working_bucket_indices_buffer,
    }
}

pub fn clean_up_extra_buffers(buffer_config: BufferConfig, extra_buffers: &mut BufferStack) {
    for _ in 0..buffer_config.num_extra_buffers() {
        extra_buffers.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_stack() {
        let mut buffers = BufferStack::new();

        let buf1 = buffers.push(10);
        assert_eq!(buf1.len(), 10);

        let [buf2, buf3] = buffers.push_two(7, 5);
        assert_eq!(buf2.len(), 7);
        assert_eq!(buf3.len(), 5);

        assert_eq!(buf2, [0, 0, 0, 0, 0, 0, 0]);
        buf2[1] = 1;

        assert!(buffers.pop());

        let buf2 = buffers.peek();

        assert_eq!(buf2, [0, 1, 0, 0, 0, 0, 0]);

        assert!(buffers.pop());
        assert!(buffers.pop());
        assert!(!buffers.pop());

        let [buf1, buf2, buf3] = buffers.push_three(7, 5, 3);
        assert_eq!(buf1.len(), 7);
        assert_eq!(buf2.len(), 5);
        assert_eq!(buf3.len(), 3);

        assert_eq!(buf2, [0, 0, 0, 0, 0]);
        buf2[1] = 1;

        let [buf1, buf2, buf3] = buffers.peek_three();
        assert_eq!(buf1.len(), 7);
        assert_eq!(buf2, [0, 1, 0, 0, 0]);
        assert_eq!(buf3.len(), 3);

        assert!(buffers.pop());
        assert!(buffers.pop());
        assert!(buffers.pop());
        assert!(!buffers.pop());
    }

    #[test]
    fn test_buffer_config_calculate_simple() {
        let text_len = 20;
        let main_buffer_len = 30;
        let num_buckets = 8;

        let buffer_config = BufferConfig::calculate(text_len, main_buffer_len, num_buckets);

        let expected_buffer_config = BufferConfig {
            is_s_type_buffer_size: 1,
            is_s_type_buffer_in_main_buffer: true,
            persistent_bucket_buffer_in_main_buffer: true,
            working_bucket_buffer_in_main_buffer: false,
        };

        assert_eq!(buffer_config, expected_buffer_config);
    }

    #[test]
    fn test_buffer_config_calculate_special_case() {
        let text_len = 20;
        let main_buffer_len = 36;
        let num_buckets = 8;

        let buffer_config = BufferConfig::calculate(text_len, main_buffer_len, num_buckets);

        let expected_buffer_config = BufferConfig {
            is_s_type_buffer_size: 1,
            is_s_type_buffer_in_main_buffer: false,
            persistent_bucket_buffer_in_main_buffer: true,
            working_bucket_buffer_in_main_buffer: true,
        };

        assert_eq!(buffer_config, expected_buffer_config);
    }
}
