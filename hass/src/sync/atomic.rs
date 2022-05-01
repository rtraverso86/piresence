use std::sync::atomic::{self, AtomicU64};
use crate::json::Id;

/// Lightweight atomic counter that generates `crate::json::Id`
/// incrementally, starting from `0`.
#[derive(Debug)]
pub struct AtomicId(AtomicU64);

impl AtomicId {
    /// Create a new `AtomicId`.
    /// The sequence of identifiers starts from `0`.
    pub fn new() -> AtomicId {
        AtomicId(AtomicU64::new(1))
    }

    /// Returns the next unique `Id`.
    /// After reaching `u64::MAX`, `next()` will start over from `0`.
    pub fn next(&self) -> Id {
        self.0.fetch_add(1, atomic::Ordering::SeqCst) as Id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_sequence(id: &AtomicId, count: Id) {
        for expected in 1..=count {
            assert_eq!(expected, id.next());
        }
    }

    #[test]
    fn new() {
        let id = AtomicId::new();
        assert_sequence(&id, 10);
    }

    #[test]
    fn independence() {
        let id1 = AtomicId::new();
        let id2 = AtomicId::new();

        assert_sequence(&id1, 100);
        assert_sequence(&id2, 100);
    }

    #[test]
    fn overflow() {
        let id = AtomicId(AtomicU64::new(u64::MAX));
        assert_eq!(u64::MAX, id.next());
        for expected in 0..10 {
            assert_eq!(expected, id.next());
        }
    }
}