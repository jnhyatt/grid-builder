use std::{
    collections::HashSet,
    hash::{BuildHasher, Hash},
};

pub trait Toggle<T> {
    fn toggle(&mut self, t: T);
}

impl<T: Eq + Hash, S: BuildHasher> Toggle<T> for HashSet<T, S> {
    fn toggle(&mut self, t: T) {
        if !self.remove(&t) {
            self.insert(t);
        }
    }
}

pub trait MinMax: Sized {
    fn min_max(self, other: Self) -> (Self, Self);
}

impl<T: Ord> MinMax for T {
    fn min_max(self, other: Self) -> (Self, Self) {
        if self < other {
            (self, other)
        } else {
            (other, self)
        }
    }
}
