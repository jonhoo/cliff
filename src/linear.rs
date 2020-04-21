use super::CliffSearch;
use core::borrow::Borrow;

/// An iterator that determines the maximum supported load by walking an iterator until the system
/// cannot keep up.
///
/// See the [crate-level documentation](..) for details.
#[derive(Debug, Clone)]
pub struct LoadIterator<I> {
    max_in: core::ops::Range<usize>,
    last: Option<usize>,
    overloaded: bool,
    iter: I,
}

impl<I, T> CliffSearch for LoadIterator<I>
where
    I: Iterator<Item = T>,
    T: Borrow<usize>,
{
    fn overloaded(&mut self) {
        self.overloaded = true;
    }

    fn estimate(&self) -> core::ops::Range<usize> {
        self.max_in.clone()
    }
}

impl<I, T> Iterator for LoadIterator<I>
where
    I: Iterator<Item = T>,
    T: Borrow<usize>,
{
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut last) = self.last {
            if self.overloaded {
                self.max_in.end = *last;
            } else {
                self.max_in.start = *last;
            }
        }

        if self.overloaded {
            return None;
        }

        let next = *self.iter.next()?.borrow();
        self.last = Some(next);
        Some(next)
    }
}

impl<I, T> From<I> for LoadIterator<I::IntoIter>
where
    I: IntoIterator<Item = T>,
    T: Borrow<usize>,
{
    fn from(v: I) -> Self {
        LoadIterator {
            max_in: 0..usize::max_value(),
            last: None,
            overloaded: false,
            iter: v.into_iter(),
        }
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
    assert_eq!(scale.estimate(), 4..usize::max_value());

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
}

#[test]
fn linear_fail() {
    let mut scale = LoadIterator::from(&[1, 2, 3, 4]);
    assert_eq!(scale.next(), Some(1));
    assert_eq!(scale.next(), Some(2));
    scale.overloaded();
    assert_eq!(scale.next(), None);
    assert_eq!(scale.estimate(), 1..2);

    // check that it continues to be terminated
    assert_eq!(scale.next(), None);
    // even after another "failed"
    scale.overloaded();
    assert_eq!(scale.next(), None);
}
