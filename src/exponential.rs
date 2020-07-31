use super::CliffSearch;

/// An iterator that determines the maximum supported load for a system by exponential search.
///
/// See the [crate-level documentation](..) for details.
#[derive(Debug, Clone)]
pub struct ExponentialCliffSearcher {
    max_in: core::ops::Range<usize>,
    start: usize,
    prev_min: usize,
    last: Option<usize>,
    fidelity: usize,
    overloaded: bool,
    done: bool,
    fill_left: bool,
}

impl ExponentialCliffSearcher {
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
            start,
            prev_min: start,
            fidelity: min_width,
            last: None,
            overloaded: false,
            done: false,
            fill_left: false,
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

    /// Ensure that samples are taken just before the cliff.
    ///
    /// If the system under test supports, say, eight million operations per second, and searches
    /// starting at 1M. The searcher will first sample 1M, 2M, 4M, 8M, 16M. When it detects that
    /// 16M is overloaded, it'll perform a binary search from the "right", running 12M, 10M, and 9M
    /// before ultimately deciding that 8M is the lower bound for the cliff.
    ///
    /// This evaluation is correct, but may yield strange-looking results when plotting as the
    /// system is likely highly loaded at 8M. In particular, a straight line drawn from the 4M
    /// sample to the 8M sample may yield a jarring visual image, and will make it hard to see what
    /// happens in the system leading up to its capacity.
    ///
    /// This method makes the searcher "fill in" extra samples to the left of the lower bound when
    /// necessary. In the case above, after finding that 8M is the lower bound, this filling would
    /// also sample 6M and 7M.
    ///
    /// Filling also respects the minimum search range width if specified with [`until`].
    pub fn fill_left(&mut self) {
        self.fill_left = true;
    }
}

impl CliffSearch for ExponentialCliffSearcher {
    fn overloaded(&mut self) {
        ExponentialCliffSearcher::overloaded(self)
    }

    fn estimate(&self) -> core::ops::Range<usize> {
        ExponentialCliffSearcher::estimate(self)
    }
}

impl Iterator for ExponentialCliffSearcher {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            if self.fill_left {
                // we've found the range in which the cliff lies: self.max_in
                // but the user has requested that we also "fill the curve" up to the min
                // by sampling some data points leading up to the cliff as well
                let diff = self.max_in.start - self.prev_min;
                if diff > self.fidelity {
                    // now just binary search between prev_min and max_in.start
                    let next = self.prev_min + diff / 2;
                    self.prev_min = next;
                    return Some(next);
                } else {
                    self.fill_left = false;
                }
            }
            return None;
        }

        if let Some(ref mut last) = self.last {
            if self.overloaded {
                // the last thing we tried failed, so it sets an upper limit for max load
                self.max_in.end = *last;
                self.overloaded = false;
            } else {
                // the last thing succeeded, so that increases the lower limit
                self.prev_min = self.max_in.start;
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
                // normally just None, but may be Some with filling
                return self.next();
            }
        } else {
            self.last = Some(self.max_in.start);
            return self.last;
        }
    }
}

#[test]
fn search_from() {
    let mut scale = ExponentialCliffSearcher::new(500);
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
    let mut scale = ExponentialCliffSearcher::until(500, 1000);
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
fn fill_search() {
    let mut scale = ExponentialCliffSearcher::until(500, 500);
    scale.fill_left();
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
    assert_eq!(scale.next(), Some(4500));
    scale.overloaded();

    // since filling is enabled, we'll also sample a few
    // points just _before_ the highest known-good target.
    assert_eq!(scale.next(), Some(3000));
    assert_eq!(scale.next(), Some(3500));

    // and then we should be done
    assert_eq!(scale.next(), None);
    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
}

#[test]
fn through_trait() {
    let mut scale = ExponentialCliffSearcher::until(500, 1000);
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
    assert_eq!(scale.estimate(), 4000..5000);
}

#[test]
fn immediate() {
    let mut scale = ExponentialCliffSearcher::new(500);
    assert_eq!(scale.next(), Some(500));
    scale.overloaded();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 500..500);
}
