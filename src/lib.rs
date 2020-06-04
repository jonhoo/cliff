//! Find the load at which a benchmark falls over.
//!
//! Most good benchmarks allow you to vary the offered load to the system, and then give you output
//! that indicate whether the system-under-test is keeping up. This could be dropped packets,
//! latency spikes, or whatever else is appropriate for the problem domain. Now, you want to find
//! out how far you can push your system until it falls over. How do you do that?
//!
//! This crate provides one answer: [exponential search]. The idea is simple: first, you double
//! offered load until the system falls over. As long as the system keeps up, you raise the lower
//! bound of your estimate for the maximum tolerated load. When the system no longer keeps up, that
//! gives you an upper limit on the throughput your system can support. At that point, you perform
//! a binary search between the upper and lower bounds, tightening the range until you reach the
//! fidelity you want.
//!
//! If you instead want to search for the _minimum_ for a given parameter, use
//! [`BinaryMinSearcher`]. It performs a binary search for a parameter between `0` and the starting
//! point you give. No exponential phase is needed for the min searchers, since `0` already bounds
//! the minimum.
//!
//! So that you can easily support manual override, the crate also provides [`LoadIterator`], which
//! implements the same interface ([`CliffSearch`]) over a pre-defined list of loads. It simply
//! stops iteration when the test runner indicates that the system is no longer keeping up through
//! [`CliffSearch::overloaded`]. To dynamically switch between these depending on user choices, use
//! `dyn CliffSearch`.
//!
//!   [exponential search]: https://en.wikipedia.org/wiki/Exponential_search
//!
//! # Examples
//!
//! ```rust
//! use cliff::ExponentialCliffSearcher;
//! # let benchmark = |load: usize| -> bool { load > 12345 };
//!
//! // First, we set the starting load. This is the initial lower bound.
//! let mut loads = ExponentialCliffSearcher::new(500);
//! while let Some(load) = loads.next() {
//!     if !benchmark(load) {
//!         loads.overloaded();
//!     }
//! }
//!
//! let supported = loads.estimate();
//! println!("maximum supported load is between {} and {}", supported.start, supported.end);
//! ```
//!
//! Stepping through the search bit by bit:
//!
//! ```rust
//! use cliff::ExponentialCliffSearcher;
//!
//! // First, we set the starting load. This is the initial lower bound.
//! let mut load = ExponentialCliffSearcher::new(500);
//! // The initial lower bound is the first load we try.
//! assert_eq!(load.next(), Some(500));
//! // Since we did not say that the system was overloaded,
//! // the iterator next produces twice the load of the previous step.
//! assert_eq!(load.next(), Some(1000));
//! // Same thing again.
//! assert_eq!(load.next(), Some(2000));
//! // Now, let's say the system did not keep up with the last load level:
//! load.overloaded();
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
//! load.overloaded();
//! // ... then the cliff must lie between 1500 and 1750, and so on.
//! // Ultimately, we reach the desired fidelity,
//! // which defaults to half the initial lower bound (here 250).
//! // At that point, no more benchmark runs are performed.
//! assert_eq!(load.next(), None);
//! // We can then ask the iterator what the final estimate is
//! assert_eq!(load.estimate(), 1500..1750);
//! ```
//!
//! Dynamically switching between search and a user-provided list:
//!
//! ```rust
//! # extern crate alloc;
//! # use alloc::{boxed::Box, vec::Vec};
//! # let user_list: Vec<usize> = Vec::new();
//! # let benchmark = |load: usize| -> bool { load > 12345 };
//! use cliff::{ExponentialCliffSearcher, CliffSearch, LoadIterator};
//!
//! let mut loads: Box<dyn CliffSearch> = if user_list.is_empty() {
//!     Box::new(ExponentialCliffSearcher::new(500))
//! } else {
//!     Box::new(LoadIterator::from(user_list))
//! };
//!
//! // from here, the strategy is the same:
//! while let Some(load) = loads.next() {
//!     if !benchmark(load) {
//!         loads.overloaded();
//!     }
//! }
//!
//! let supported = loads.estimate();
//! println!("maximum supported load is between {} and {}", supported.start, supported.end);
//! ```
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![no_std]

mod binmin;
mod exponential;
mod linear;

pub use binmin::BinaryMinSearcher;
pub use exponential::ExponentialCliffSearcher;
pub use linear::LoadIterator;

/// A class of type that can estimate the performance cliff for a system.
pub trait CliffSearch: Iterator<Item = usize> {
    /// Indicate that the system could not keep up with the previous load factor yielded by
    /// [`Iterator::next`].
    ///
    /// This will affect what value the next call to [`Iterator::next`] yields.
    fn overloaded(&mut self);

    /// Give the current estimate of the maximum load the system-under-test can support.
    fn estimate(&self) -> core::ops::Range<usize>;
}
