use super::CliffSearch;

/// An iterator that determines the maximum supported load for a system by binary search.
///
/// See the [crate-level documentation](..) for details.
#[derive(Debug, Clone)]
pub struct BinaryCliffSearcher {
    max_in: core::ops::Range<usize>,
    last: Option<usize>,
    fidelity: usize,
    overloaded: bool,
    done: bool,
}

impl BinaryCliffSearcher {
    /// Perform a load search starting at `start`, and ending when the maximum load has been
    /// determined to within a range of `start / 2`.
    pub fn new(start: usize) -> Self {
        Self::until(start, start / 2)
    }

    /// Perform a load search starting at `start`, and ending when the maximum load has been
    /// determined to within a range of `min_width`.
    pub fn until(start: usize, min_width: usize) -> Self {
        Self {
            max_in: start..usize::max_value(),
            fidelity: min_width,
            last: None,
            overloaded: false,
            done: false,
        }
    }

    // NOTE: we provide inherent methods for CliffSearch so that those who do not need LoadIterator
    // do not need to think about the trait at all.

    /// Indicate that the system could not keep up with the previous load factor yielded by
    /// [`Iterator::next`].
    ///
    /// This will affect what value the next call to [`Iterator::next`] yields.
    ///
    /// This provides [`CliffSearch::overloaded`] without having to `use` the trait.
    pub fn overloaded(&mut self) {
        self.overloaded = true;
    }

    /// Give the current estimate of the maximum load the system-under-test can support.
    ///
    /// This provides [`CliffSearch::estimate`] without having to `use` the trait.
    pub fn estimate(&self) -> core::ops::Range<usize> {
        self.max_in.clone()
    }
}

impl CliffSearch for BinaryCliffSearcher {
    fn overloaded(&mut self) {
        BinaryCliffSearcher::overloaded(self)
    }

    fn estimate(&self) -> core::ops::Range<usize> {
        BinaryCliffSearcher::estimate(self)
    }
}

impl Iterator for BinaryCliffSearcher {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if let Some(ref mut last) = self.last {
            if self.overloaded {
                // the last thing we tried failed, so it sets an upper limit for max load
                self.max_in.end = *last;
                self.overloaded = false;
            } else {
                // the last thing succeeded, so that increases the lower limit
                self.max_in.start = *last;
            }

            let next = if self.max_in.end == usize::max_value() {
                // no upper limit, so exponential search
                2 * self.max_in.start
            } else {
                // bisect the range
                self.max_in.start + (self.max_in.end - self.max_in.start) / 2
            };

            // we only care about the max down to `fidelity`
            if self.max_in.end - self.max_in.start > self.fidelity {
                *last = next;
                Some(next)
            } else {
                self.done = true;
                None
            }
        } else {
            self.last = Some(self.max_in.start);
            return self.last;
        }
    }
}

#[test]
fn search_from() {
    let mut scale = BinaryCliffSearcher::new(500);
    assert_eq!(scale.next(), Some(500));
    assert_eq!(scale.next(), Some(1000));
    assert_eq!(scale.next(), Some(2000));
    assert_eq!(scale.next(), Some(4000));
    scale.overloaded();
    assert_eq!(scale.next(), Some(3000));
    assert_eq!(scale.next(), Some(3500));
    scale.overloaded();
    assert_eq!(scale.next(), Some(3250));
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 3250..3500);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
    // and the estimate is still the same
    assert_eq!(scale.estimate(), 3250..3500);
}

#[test]
fn search_from_until() {
    let mut scale = BinaryCliffSearcher::until(500, 1000);
    assert_eq!(scale.next(), Some(500));
    assert_eq!(scale.next(), Some(1000));
    assert_eq!(scale.next(), Some(2000));
    assert_eq!(scale.next(), Some(4000));
    assert_eq!(scale.next(), Some(8000));
    scale.overloaded();
    assert_eq!(scale.next(), Some(6000));
    scale.overloaded();
    assert_eq!(scale.next(), Some(5000));
    scale.overloaded();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 4000..5000);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
    // and the estimate is still the same
    assert_eq!(scale.estimate(), 4000..5000);
}

#[test]
fn through_trait() {
    let mut scale = BinaryCliffSearcher::until(500, 1000);
    let scale: &mut dyn CliffSearch = &mut scale;
    assert_eq!(scale.next(), Some(500));
    assert_eq!(scale.next(), Some(1000));
    assert_eq!(scale.next(), Some(2000));
    assert_eq!(scale.next(), Some(4000));
    assert_eq!(scale.next(), Some(8000));
    scale.overloaded();
    assert_eq!(scale.next(), Some(6000));
    scale.overloaded();
    assert_eq!(scale.next(), Some(5000));
    scale.overloaded();
    assert_eq!(scale.next(), None);
}
