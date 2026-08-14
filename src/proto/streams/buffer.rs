use slab::Slab;

/// Buffers frames for multiple streams.
#[derive(Debug)]
pub struct Buffer<T> {
    slab: Slab<Slot<T>>,
}

/// A sequence of frames in a `Buffer`
#[derive(Debug)]
pub struct Deque {
    indices: Option<Indices>,
}

/// Tracks the head & tail for a sequence of frames in a `Buffer`.
#[derive(Debug, Default, Copy, Clone)]
struct Indices {
    head: usize,
    tail: usize,
}

#[derive(Debug)]
struct Slot<T> {
    value: T,
    next: Option<usize>,
}

impl<T> Buffer<T> {
    pub fn new() -> Self {
        Buffer { slab: Slab::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.slab.is_empty()
    }
}

impl Deque {
    pub fn new() -> Self {
        Deque { indices: None }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_none()
    }

    pub fn push_back<T>(&mut self, buf: &mut Buffer<T>, value: T) {
        let key = buf.slab.insert(Slot { value, next: None });

        match self.indices {
            Some(ref mut idxs) => {
                buf.slab[idxs.tail].next = Some(key);
                idxs.tail = key;
            }
            None => {
                self.indices = Some(Indices {
                    head: key,
                    tail: key,
                });
            }
        }
    }

    pub fn push_front<T>(&mut self, buf: &mut Buffer<T>, value: T) {
        let key = buf.slab.insert(Slot { value, next: None });

        match self.indices {
            Some(ref mut idxs) => {
                buf.slab[key].next = Some(idxs.head);
                idxs.head = key;
            }
            None => {
                self.indices = Some(Indices {
                    head: key,
                    tail: key,
                });
            }
        }
    }

    pub fn pop_front<T>(&mut self, buf: &mut Buffer<T>) -> Option<T> {
        match self.indices {
            Some(mut idxs) => {
                let mut slot = buf.slab.remove(idxs.head);

                if idxs.head == idxs.tail {
                    assert!(slot.next.is_none());
                    self.indices = None;
                } else {
                    idxs.head = slot.next.take().unwrap();
                    self.indices = Some(idxs);
                }

                Some(slot.value)
            }
            None => None,
        }
    }

    /// Remove and return the first value matching `pred`, leaving other entries
    /// in order.
    pub fn take_first_if<T, F>(&mut self, buf: &mut Buffer<T>, mut pred: F) -> Option<T>
    where
        F: FnMut(&T) -> bool,
    {
        let mut idxs = self.indices?;
        let mut prev: Option<usize> = None;
        let mut cur = idxs.head;

        loop {
            if pred(&buf.slab[cur].value) {
                let slot = buf.slab.remove(cur);
                if let Some(p) = prev {
                    buf.slab[p].next = slot.next;
                    if idxs.tail == cur {
                        idxs.tail = p;
                    }
                    self.indices = Some(idxs);
                } else {
                    match slot.next {
                        Some(n) => {
                            idxs.head = n;
                            self.indices = Some(idxs);
                        }
                        None => self.indices = None,
                    }
                }
                return Some(slot.value);
            }

            match buf.slab[cur].next {
                Some(n) => {
                    prev = Some(cur);
                    cur = n;
                }
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_first_if_extracts_middle_keeps_order() {
        let mut buf = Buffer::new();
        let mut q = Deque::new();
        q.push_back(&mut buf, 1);
        q.push_back(&mut buf, 2);
        q.push_back(&mut buf, 3);
        q.push_back(&mut buf, 4);

        assert_eq!(q.take_first_if(&mut buf, |v| *v == 3), Some(3));
        assert_eq!(q.pop_front(&mut buf), Some(1));
        assert_eq!(q.pop_front(&mut buf), Some(2));
        assert_eq!(q.pop_front(&mut buf), Some(4));
        assert_eq!(q.pop_front(&mut buf), None);
    }

    #[test]
    fn take_first_if_head_and_tail() {
        let mut buf = Buffer::new();
        let mut q = Deque::new();
        q.push_back(&mut buf, 1);
        q.push_back(&mut buf, 2);
        q.push_back(&mut buf, 3);

        assert_eq!(q.take_first_if(&mut buf, |v| *v == 1), Some(1));
        assert_eq!(q.take_first_if(&mut buf, |v| *v == 3), Some(3));
        assert_eq!(q.pop_front(&mut buf), Some(2));
        assert_eq!(q.pop_front(&mut buf), None);
    }

    #[test]
    fn take_first_if_none() {
        let mut buf = Buffer::new();
        let mut q = Deque::new();
        q.push_back(&mut buf, 1);
        assert_eq!(q.take_first_if(&mut buf, |v| *v == 9), None);
        assert_eq!(q.pop_front(&mut buf), Some(1));
    }
}
