pub struct Ringbuffer {
    write_pos: usize, // Position to write next value to
}

impl Ringbuffer {
    pub fn new() -> Ringbuffer {
        let write_pos = 0;
        Ringbuffer{write_pos}
    }

    pub fn init<T: Default >(&mut self, buffer: &mut [T]) {
        for i in 0..buffer.len() {
            buffer[i] = T::default();
        }
    }

    pub fn add<T>(&mut self, buffer: &mut [T], value: T) {
        let len = buffer.len();
        self.write_pos = Ringbuffer::inc(self.write_pos, len);
        buffer[self.write_pos] = value;
    }

    /** Get a value from the ringbuffer.
     * 
     * index is the age of the value, the higher the index, the further back from the write
     * position it is.
     */
    pub fn get<T: Copy>(&mut self, buffer: &[T], mut index: usize) -> T {
        let len = buffer.len();
        while index >= len {
            index -= len;
        }
        let index = Ringbuffer::sub(self.write_pos, index, len);
        buffer[index]
    }

    fn inc(mut val: usize, max: usize) -> usize {
        val += 1;
        val = if val >= max { val - max } else { val };
        val
    }

    fn sub(mut value: usize, dec: usize, len: usize) -> usize {
        if value < dec {
            value += len;
        }
        value - dec
    }
}

#[cfg(test)]
#[test]
fn test_ringbuff() {
    let mut buffer = [0.0; 4];
    let mut rb = Ringbuffer::new();
    rb.init(&mut buffer);

    rb.add(&mut buffer, 1.0);
    rb.add(&mut buffer, 2.0);
    rb.add(&mut buffer, 3.0);
    rb.add(&mut buffer, 4.0);

    assert_eq!(rb.get(&mut buffer, 0), 4.0);
    assert_eq!(rb.get(&mut buffer, 1), 3.0);
    assert_eq!(rb.get(&mut buffer, 2), 2.0);
    assert_eq!(rb.get(&mut buffer, 3), 1.0);
    assert_eq!(rb.get(&mut buffer, 4), 4.0);

    rb.add(&mut buffer, 5.0);
    assert_eq!(rb.get(&mut buffer, 0), 5.0);
    assert_eq!(rb.get(&mut buffer, 1), 4.0);
    assert_eq!(rb.get(&mut buffer, 2), 3.0);
    assert_eq!(rb.get(&mut buffer, 3), 2.0);
}
