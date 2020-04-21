//! Find the load at which a benchmark falls over.
//!
//! Most good benchmarks allow you to vary the offered load to the system, and then give you output
//! that indicate whether the system-under-test is keeping up. This could be dropped packets,
//! latency spikes, or whatever else is appropriate for the problem domain. Now, you want to find
//! out how far you can push your system until it falls over. How do you do that?
//!
//! This crate provides one answer: binary search. The idea is simple: first, you double offered
//! load until the system falls over. As long as the system keeps up, you raise the lower bound of
//! your estimate for the maximum tolerated load. When the system no longer keeps up, that gives
//! you an upper limit on the throughput your system can support. At that point, you perform a
//! binary search between the upper and lower bounds, tightening the range until you reach the
//! fidelity you want.
//!
//! So that you can easily support manual override, [`LoadIterator`] also supports a linear mode.
//! In this mode, cliff just walks a pre-defined list of loads, and stops when the test runner
//! indicates that the system is no longer keeping up.
//!
//! # Examples
//!
//! ```rust
//! use cliff::LoadIterator;
//!
//! // First, we set the starting load. This is the initial lower bound.
//! let mut load = LoadIterator::search_from(500);
//! // The initial lower bound is the first load we try.
//! assert_eq!(load.next(), Some(500));
//! // Since we did not say that the system was overloaded,
//! // the iterator next produces twice the load of the previous step.
//! assert_eq!(load.next(), Some(1000));
//! // Same thing again.
//! assert_eq!(load.next(), Some(2000));
//! // Now, let's say the system did not keep up with the last load level:
//! load.failed();
//! // At this point, cliff will begin a binary search between
//! // 1000 (the highest supported load)
//! // and
//! // 2000 (the lowest unsupported load).
//! // The middle of that range is 1500, so that's what it'll try.
//! assert_eq!(load.next(), Some(1500));
//! // Let's say that succeeded.
//! // That means the cliff must lie between 1500 and 2000, so we try 1750:
//! assert_eq!(load.next(), Some(1750));
//! // And if that failed ...
//! load.failed();
//! // ... then the cliff must lie between 1500 and 1750, and so on.
//! // Ultimately, we reach the desired fidelity,
//! // which defaults to half the initial lower bound (here 250).
//! // At that point, no more benchmark runs are performed.
//! assert_eq!(load.next(), None);
//! ```
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![no_std]

use core::borrow::Borrow;

/// An iterator that helps determine maximum supported load for a system.
///
/// See the [crate-level documentation](..) for details.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LoadIterator<I>(Inner<I>);

#[derive(Debug, Clone)]
enum Inner<I> {
    Iter {
        max_in: core::ops::Range<usize>,
        last: Option<usize>,
        failed: bool,
        iter: I,
    },
    Search {
        max_in: core::ops::Range<usize>,
        last: Option<usize>,
        fidelity: usize,
        failed: bool,
        done: bool,
    },
}

impl LoadIterator<core::slice::Iter<'static, usize>> {
    /// Perform a load search starting at `start`, and ending when the maximum load has been
    /// determined to within a range of `start / 2`.
    pub fn search_from(start: usize) -> Self {
        Self::search_from_until(start, start / 2)
    }

    /// Perform a load search starting at `start`, and ending when the maximum load has been
    /// determined to within a range of `min_width`.
    pub fn search_from_until(start: usize, min_width: usize) -> Self {
        LoadIterator(Inner::Search {
            max_in: start..usize::max_value(),
            fidelity: min_width,
            last: None,
            failed: false,
            done: false,
        })
    }
}

impl<I> LoadIterator<I> {
    /// Indicate that the system could not keep up with the previous load factor yielded by
    /// [`Iterator::next`].
    ///
    /// This will affect what value the next call to [`Iterator::next`] yields.
    pub fn failed(&mut self) {
        match self.0 {
            Inner::Search { ref mut failed, .. } | Inner::Iter { ref mut failed, .. } => {
                *failed = true;
            }
        }
    }

    /// Give the current estimate of the maximum load the system-under-test can support.
    pub fn range(&self) -> core::ops::Range<usize> {
        match self.0 {
            Inner::Search { ref max_in, .. } | Inner::Iter { ref max_in, .. } => max_in.clone(),
        }
    }
}

impl<I, T> Iterator for LoadIterator<I>
where
    I: Iterator<Item = T>,
    T: Borrow<usize>,
{
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<I, T> Iterator for Inner<I>
where
    I: Iterator<Item = T>,
    T: Borrow<usize>,
{
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Inner::Iter {
                ref mut max_in,
                ref mut failed,
                ref mut last,
                ref mut iter,
            } => {
                if let Some(ref mut last) = *last {
                    if *failed {
                        max_in.end = *last;
                    } else {
                        max_in.start = *last;
                    }
                }

                if *failed {
                    return None;
                }

                let next = *iter.next()?.borrow();
                *last = Some(next);
                Some(next)
            }
            Inner::Search {
                ref mut max_in,
                ref mut failed,
                ref mut last,
                ref mut done,
                fidelity,
            } => {
                if *done {
                    return None;
                }

                if let Some(ref mut last) = *last {
                    if *failed {
                        // the last thing we tried failed, so it sets an upper limit for max load
                        max_in.end = *last;
                        *failed = false;
                    } else {
                        // the last thing succeeded, so that increases the lower limit
                        max_in.start = *last;
                    }

                    let next = if max_in.end == usize::max_value() {
                        // no upper limit, so exponential search
                        2 * max_in.start
                    } else {
                        // bisect the range
                        max_in.start + (max_in.end - max_in.start) / 2
                    };

                    // we only care about the max down to `fidelity`
                    if max_in.end - max_in.start > fidelity {
                        *last = next;
                        Some(next)
                    } else {
                        *done = true;
                        None
                    }
                } else {
                    *last = Some(max_in.start);
                    return *last;
                }
            }
        }
    }
}

impl<I, T> From<I> for LoadIterator<I::IntoIter>
where
    I: IntoIterator<Item = T>,
    T: Borrow<usize>,
{
    fn from(v: I) -> Self {
        LoadIterator(Inner::Iter {
            max_in: 0..usize::max_value(),
            last: None,
            failed: false,
            iter: v.into_iter(),
        })
    }
}

#[test]
fn linear_nofail() {
    let mut scale = LoadIterator::from(&[1, 2, 3, 4]);
    assert_eq!(scale.next(), Some(1));
    assert_eq!(scale.next(), Some(2));
    assert_eq!(scale.next(), Some(3));
    assert_eq!(scale.next(), Some(4));
    assert_eq!(scale.next(), None);
    assert_eq!(scale.range(), 4..usize::max_value());

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.failed();
    assert_eq!(scale.next(), None);
}

#[test]
fn linear_fail() {
    let mut scale = LoadIterator::from(&[1, 2, 3, 4]);
    assert_eq!(scale.next(), Some(1));
    assert_eq!(scale.next(), Some(2));
    scale.failed();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.range(), 1..2);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.failed();
    assert_eq!(scale.next(), None);
}

#[test]
fn search_from() {
    let mut scale = LoadIterator::search_from(500);
    assert_eq!(scale.next(), Some(500));
    assert_eq!(scale.next(), Some(1000));
    assert_eq!(scale.next(), Some(2000));
    assert_eq!(scale.next(), Some(4000));
    scale.failed();
    assert_eq!(scale.next(), Some(3000));
    assert_eq!(scale.next(), Some(3500));
    scale.failed();
    assert_eq!(scale.next(), Some(3250));
    assert_eq!(scale.next(), None);
    assert_eq!(scale.range(), 3250..3500);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.failed();
    assert_eq!(scale.next(), None);
    // and the range is still the same
    assert_eq!(scale.range(), 3250..3500);
}

#[test]
fn search_from_until() {
    let mut scale = LoadIterator::search_from_until(500, 1000);
    assert_eq!(scale.next(), Some(500));
    assert_eq!(scale.next(), Some(1000));
    assert_eq!(scale.next(), Some(2000));
    assert_eq!(scale.next(), Some(4000));
    assert_eq!(scale.next(), Some(8000));
    scale.failed();
    assert_eq!(scale.next(), Some(6000));
    scale.failed();
    assert_eq!(scale.next(), Some(5000));
    scale.failed();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.range(), 4000..5000);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.failed();
    assert_eq!(scale.next(), None);
    // and the range is still the same
    assert_eq!(scale.range(), 4000..5000);
}
