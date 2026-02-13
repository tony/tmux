use std::collections::VecDeque;

pub struct RingBuffer<T> {
    inner: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if self.inner.len() == self.capacity {
            self.inner.pop_front();
        }
        self.inner.push_back(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_new() {
        let rb: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(rb.capacity, 5);
        assert!(rb.inner.is_empty());
    }

    #[test]
    fn test_ring_buffer_push() {
        let mut rb = RingBuffer::new(2);
        rb.push(1);
        assert_eq!(rb.inner.len(), 1);
        rb.push(2);
        assert_eq!(rb.inner.len(), 2);
        rb.push(3);
        assert_eq!(rb.inner.len(), 2);
        assert_eq!(rb.inner[0], 2);
        assert_eq!(rb.inner[1], 3);
    }

    #[test]
    fn test_ring_buffer_zero_cap() {
        let mut rb = RingBuffer::new(0);
        rb.push(1);
        assert!(rb.inner.is_empty());
    }

    #[test]
    fn test_ring_buffer_large() {
        let mut rb = RingBuffer::new(100);
        for i in 0..200 {
            rb.push(i);
        }
        assert_eq!(rb.inner.len(), 100);
        assert_eq!(rb.inner.back(), Some(&199));
    }

    #[test] fn test_rb_1() { let mut rb = RingBuffer::new(1); rb.push(10); assert_eq!(rb.inner[0], 10); }
    #[test] fn test_rb_2() { let mut rb = RingBuffer::new(1); rb.push(10); rb.push(20); assert_eq!(rb.inner[0], 20); }
    #[test] fn test_rb_3() { let rb: RingBuffer<u8> = RingBuffer::new(10); assert!(rb.inner.is_empty()); }
    #[test] fn test_rb_4() { let mut rb = RingBuffer::new(5); rb.push(1); rb.push(2); assert_eq!(rb.inner.len(), 2); }
    #[test] fn test_rb_5() { let mut rb = RingBuffer::new(2); rb.push(1); rb.push(2); rb.push(3); assert_eq!(rb.inner.front(), Some(&2)); }
}
