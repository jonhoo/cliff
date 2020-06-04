use super::CliffSearch;

/// An iterator that determines the _minimum_ value of a system parameter by binary search.
///
/// ```rust
/// use cliff::BinaryMinSearcher;
///
/// // First, we set the starting value for the parameter.
/// // This is the initial upper bound.
/// let mut limit = BinaryMinSearcher::until(512, 32);
/// // The initial upper bound is the first value we try.
/// assert_eq!(limit.next(), Some(512));
/// // Since we did not say that the system was overloaded,
/// // the iterator next produces half the value of the previous step.
/// assert_eq!(limit.next(), Some(256));
/// // Same thing again.
/// assert_eq!(limit.next(), Some(128));
/// // Now, let's say the system did not keep up with the last parameter value:
/// limit.overloaded();
/// // 128 is now a known _lower_ bound for the value, so the iteration
/// // continues the binary search between 128 and 256 (the last known-good value).
/// assert_eq!(limit.next(), Some(192));
/// // Let's say that succeeded.
/// // That means the cliff must lie between 128 and 192, so we try 160:
/// assert_eq!(limit.next(), Some(160));
/// // And if that failed ...
/// limit.overloaded();
/// // ... then the cliff must lie between 160 and 192, and so on.
/// // Ultimately, we reach the desired fidelity, which we set to 32.
/// // At that point, no more benchmark runs are performed.
/// assert_eq!(limit.next(), None);
/// // We can then ask the iterator what the final estimate is
/// assert_eq!(limit.estimate(), 160..192);
/// ```
///
/// See also the [crate-level documentation](..) for details.
#[derive(Debug, Clone)]
pub struct BinaryMinSearcher {
    min_in: core::ops::Range<usize>,
    last: Option<usize>,
    fidelity: usize,
    overloaded: bool,
    done: bool,
}

impl BinaryMinSearcher {
    /// Perform a minimum search starting at `start`, and ending when the minimum has been
    /// determined to within a range of `min_width`.
    pub fn until(start: usize, min_width: usize) -> Self {
        Self {
            min_in: 0..start,
            fidelity: min_width,
            last: None,
            overloaded: false,
            done: false,
        }
    }

    // NOTE: we provide inherent methods for CliffSearch so that those who do not need LoadIterator
    // do not need to think about the trait at all.

    /// Indicate that the system could not keep up with the previous parameter yielded by
    /// [`Iterator::next`].
    ///
    /// This will affect what value the next call to [`Iterator::next`] yields.
    ///
    /// This provides [`CliffSearch::overloaded`] without having to `use` the trait.
    pub fn overloaded(&mut self) {
        self.overloaded = true;
    }

    /// Give the current estimate of the minimum parameter load the system-under-test can support.
    ///
    /// This provides [`CliffSearch::estimate`] without having to `use` the trait.
    pub fn estimate(&self) -> core::ops::Range<usize> {
        self.min_in.clone()
    }
}

impl CliffSearch for BinaryMinSearcher {
    fn overloaded(&mut self) {
        BinaryMinSearcher::overloaded(self)
    }

    fn estimate(&self) -> core::ops::Range<usize> {
        BinaryMinSearcher::estimate(self)
    }
}

impl Iterator for BinaryMinSearcher {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if let Some(ref mut last) = self.last {
            if self.overloaded {
                // the last thing we tried failed, so it sets a lower limit for min
                self.min_in.start = *last;
                self.overloaded = false;
            } else {
                // the last thing succeeded, so that lowers the upper limit
                self.min_in.end = *last;
            }

            // bisect the range
            let next = self.min_in.start + (self.min_in.end - self.min_in.start) / 2;

            // we only care about the min down to `fidelity`
            if self.min_in.end - self.min_in.start > self.fidelity {
                *last = next;
                Some(next)
            } else {
                self.done = true;
                None
            }
        } else {
            self.last = Some(self.min_in.end);
            return self.last;
        }
    }
}

#[test]
fn search_from_until() {
    let mut scale = BinaryMinSearcher::until(1024, 8);
    assert_eq!(scale.next(), Some(1024));
    assert_eq!(scale.next(), Some(512));
    assert_eq!(scale.next(), Some(256));
    assert_eq!(scale.next(), Some(128));
    assert_eq!(scale.next(), Some(64));
    scale.overloaded();
    assert_eq!(scale.next(), Some(96));
    assert_eq!(scale.next(), Some(80));
    scale.overloaded();
    assert_eq!(scale.next(), Some(88));
    assert_eq!(scale.next(), None);
    // system could handle 88, so that's the upper limit
    // system could not handle 80, so that's the lower limit
    assert_eq!(scale.estimate(), 80..88);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
    // and the estimate is still the same
    assert_eq!(scale.estimate(), 80..88);
}

#[test]
fn through_trait() {
    let mut scale = BinaryMinSearcher::until(1024, 8);
    let scale: &mut dyn CliffSearch = &mut scale;
    assert_eq!(scale.next(), Some(1024));
    assert_eq!(scale.next(), Some(512));
    assert_eq!(scale.next(), Some(256));
    assert_eq!(scale.next(), Some(128));
    assert_eq!(scale.next(), Some(64));
    scale.overloaded();
    assert_eq!(scale.next(), Some(96));
    assert_eq!(scale.next(), Some(80));
    scale.overloaded();
    assert_eq!(scale.next(), Some(88));
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 80..88);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
    // and the estimate is still the same
    assert_eq!(scale.estimate(), 80..88);
}

#[test]
fn immediate() {
    let mut scale = BinaryMinSearcher::until(1024, 8);
    assert_eq!(scale.next(), Some(1024));
    scale.overloaded();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 1024..1024);
}
