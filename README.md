[![Crates.io](https://img.shields.io/crates/v/cliff.svg)](https://crates.io/crates/cliff)
[![Documentation](https://docs.rs/cliff/badge.svg)](https://docs.rs/cliff/)
[![Build Status](https://dev.azure.com/jonhoo/jonhoo/_apis/build/status/cliff?branchName=master)](https://dev.azure.com/jonhoo/jonhoo/_build/latest?definitionId=24&branchName=master)
[![Codecov](https://codecov.io/github/jonhoo/cliff/coverage.svg?branch=master)](https://codecov.io/gh/jonhoo/cliff)

Find the load at which a benchmark falls over.

Most good benchmarks allow you to vary the offered load to the system,
and then give you output that indicate whether the system-under-test is
keeping up. This could be dropped packets, latency spikes, or whatever
else is appropriate for the problem domain. Now, you want to find out
how far you can push your system until it falls over. How do you do
that?

This crate provides one answer: binary search. The idea is simple:
first, you double offered load until the system falls over. As long as
the system keeps up, you raise the lower bound of your estimate for the
maximum tolerated load. When the system no longer keeps up, that gives
you an upper limit on the throughput your system can support. At that
point, you perform a binary search between the upper and lower bounds,
tightening the range until you reach the fidelity you want.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
