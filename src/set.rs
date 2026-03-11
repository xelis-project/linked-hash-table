//! [`LinkedHashSet`] - an insertion-ordered hash set.
//!
//! Implemented as a thin wrapper around [`LinkedHashMap<T, ()>`].
//!
//! | Iterator type | Created by | Yields |
//! |---------------|-----------|--------|
//! | [`SetIter`] | [`iter`] | `&T` |
//! | [`SetDrain`] | [`drain`] | `T` - empties the set |
//! | [`SetIntoIter`] | [`into_iter`] | `T` - consumes the set |
//!
//! [`iter`]: LinkedHashSet::iter
//! [`drain`]: LinkedHashSet::drain
//! [`into_iter`]: LinkedHashSet::into_iter

use std::borrow::Borrow;
use std::fmt;
use std::hash::{BuildHasher, Hash, RandomState};

use crate::LinkedHashMap;
use crate::iter::{Drain as MapDrain, IntoIter as MapIntoIter, Keys};

/// A hash set that preserves **insertion order** and exposes a
/// [`VecDeque`]-like API with [`insert_back`], [`insert_front`],
/// [`pop_front`], and [`pop_back`].
///
/// Implemented as a thin wrapper around [`LinkedHashMap<T, ()>`].
///
/// Elements require only [`Hash`] + [`Eq`].  They do **not** need to be
/// [`Clone`] or [`Copy`] — including heap-owning types such as `String`,
/// `Vec<T>`, or `Box<T>`.
///
/// ## Ordering contract
///
/// `insert_back` and `insert_front` on an element that **already exists**
/// are a **no-op** - the element keeps its current position.  Use
/// [`move_to_back`] / [`move_to_front`] to explicitly reorder an element.
///
/// ## Example
///
/// ```rust
/// use linked_hash_table::LinkedHashSet;
///
/// let mut s = LinkedHashSet::new();
/// s.insert_back("a");
/// s.insert_back("b");
/// s.insert_back("c");
/// assert_eq!(s.pop_front(), Some("a"));
/// assert_eq!(s.pop_back(),  Some("c"));
/// ```
///
/// [`VecDeque`]: std::collections::VecDeque
/// [`insert_back`]: LinkedHashSet::insert_back
/// [`insert_front`]: LinkedHashSet::insert_front
/// [`move_to_back`]: LinkedHashSet::move_to_back
/// [`move_to_front`]: LinkedHashSet::move_to_front
pub struct LinkedHashSet<T, S = RandomState> {
    map: LinkedHashMap<T, (), S>,
}

impl<T> LinkedHashSet<T> {
    /// Creates an empty `LinkedHashSet`.
    pub fn new() -> Self {
        Self {
            map: LinkedHashMap::new(),
        }
    }

    /// Creates an empty `LinkedHashSet` with the specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: LinkedHashMap::with_capacity(capacity),
        }
    }
}

impl<T, S> LinkedHashSet<T, S> {
    /// Creates an empty `LinkedHashSet` using the supplied hasher builder.
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: LinkedHashMap::with_hasher(hash_builder),
        }
    }

    /// Creates an empty `LinkedHashSet` with the given capacity and hasher.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: LinkedHashMap::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    /// Returns the number of elements in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns the number of elements the set can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Returns a reference to the set's [`BuildHasher`].
    #[inline]
    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }
}

impl<T, S> LinkedHashSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    /// Inserts `value` at the **back** (most-recently-inserted end).
    ///
    /// Returns `true` if the value was newly inserted.  If the value already
    /// exists its position is **preserved** (no-op) and `false` is returned.
    pub fn insert_back(&mut self, value: T) -> bool {
        self.map.insert_back(value, ()).is_none()
    }

    /// Inserts `value` at the **front** (least-recently-inserted end).
    ///
    /// Returns `true` if the value was newly inserted.  If the value already
    /// exists its position is **preserved** (no-op) and `false` is returned.
    pub fn insert_front(&mut self, value: T) -> bool {
        self.map.insert_front(value, ()).is_none()
    }

    /// Alias for [`insert_back`] - matches the [`HashSet::insert`] signature.
    ///
    /// [`insert_back`]: LinkedHashSet::insert_back
    /// [`HashSet::insert`]: std::collections::HashSet::insert
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.insert_back(value)
    }

    /// Returns `true` if the set contains `value`.
    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(value)
    }

    /// Returns a reference to the element in the set that equals `value`,
    /// or `None` if it is not present.
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_key_value(value).map(|(k, _)| k)
    }

    /// Returns a reference to the **front** (oldest) element, or `None` if
    /// the set is empty.
    pub fn front(&self) -> Option<&T> {
        self.map.front().map(|(k, _)| k)
    }

    /// Returns a reference to the **back** (most recently inserted) element,
    /// or `None` if the set is empty.
    pub fn back(&self) -> Option<&T> {
        self.map.back().map(|(k, _)| k)
    }

    /// Removes and returns the **front** (oldest) element, or `None`.
    pub fn pop_front(&mut self) -> Option<T> {
        self.map.pop_front().map(|(k, _)| k)
    }

    /// Removes and returns the **back** (newest) element, or `None`.
    pub fn pop_back(&mut self) -> Option<T> {
        self.map.pop_back().map(|(k, _)| k)
    }

    /// Removes `value` from the set.
    /// Returns `true` if the element was present.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove(value).is_some()
    }

    /// Removes the element equal to `value` and returns it, if present.
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove_entry(value).map(|(k, _)| k)
    }

    /// Retains only the elements for which `f` returns `true`.
    /// Elements are visited in **insertion order** (front → back).
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.map.retain(|k, _| f(k));
    }

    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Moves `value` to the **back** of the ordering.
    /// Returns `true` if the element was found.
    pub fn move_to_back<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.move_to_back(value)
    }

    /// Moves `value` to the **front** of the ordering.
    /// Returns `true` if the element was found.
    pub fn move_to_front<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.move_to_front(value)
    }

    /// Removes all elements in **insertion order**, returning them via an
    /// iterator.  The set is empty after this call (even if the iterator is
    /// dropped before it is fully consumed).
    pub fn drain(&mut self) -> SetDrain<'_, T> {
        SetDrain {
            inner: self.map.drain(),
        }
    }

    /// Returns `true` if every element of `self` is also contained in `other`.
    pub fn is_subset<S2: BuildHasher>(&self, other: &LinkedHashSet<T, S2>) -> bool {
        self.len() <= other.len() && self.iter().all(|v| other.contains(v))
    }

    /// Returns `true` if every element of `other` is also contained in `self`.
    pub fn is_superset<S2: BuildHasher>(&self, other: &LinkedHashSet<T, S2>) -> bool {
        other.is_subset(self)
    }

    /// Returns `true` if `self` and `other` share no elements.
    pub fn is_disjoint<S2: BuildHasher>(&self, other: &LinkedHashSet<T, S2>) -> bool {
        if self.len() <= other.len() {
            self.iter().all(|v| !other.contains(v))
        } else {
            other.iter().all(|v| !self.contains(v))
        }
    }
}

impl<T, S> LinkedHashSet<T, S> {
    /// Returns an iterator over elements in **insertion order**.
    pub fn iter<'a>(&'a self) -> SetIter<'a, T> {
        SetIter {
            inner: self.map.keys(),
        }
    }
}

impl<T> Default for LinkedHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug, S> fmt::Debug for LinkedHashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: PartialEq, S1, S2> PartialEq<LinkedHashSet<T, S2>> for LinkedHashSet<T, S1> {
    /// Two sets are equal only when they contain **the same elements in the
    /// same order**.
    fn eq(&self, other: &LinkedHashSet<T, S2>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: PartialEq + Eq, S> Eq for LinkedHashSet<T, S> {}

impl<T: Clone + Hash + Eq, S: BuildHasher + Clone> Clone for LinkedHashSet<T, S> {
    fn clone(&self) -> Self {
        let mut new_set = Self::with_capacity_and_hasher(self.len(), self.map.hasher().clone());
        for v in self.iter() {
            new_set.insert_back(v.clone());
        }
        new_set
    }
}

impl<T: Hash + Eq> FromIterator<T> for LinkedHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        let mut set = Self::with_capacity(lower);
        for v in iter {
            set.insert_back(v);
        }
        set
    }
}

impl<T: Hash + Eq, S: BuildHasher> Extend<T> for LinkedHashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert_back(v);
        }
    }
}

/// Consuming iterator: yields all elements in insertion order.
impl<T, S> IntoIterator for LinkedHashSet<T, S> {
    type Item = T;
    type IntoIter = SetIntoIter<T>;

    fn into_iter(self) -> SetIntoIter<T> {
        SetIntoIter {
            inner: self.map.into_iter(),
        }
    }
}

/// Shared-reference iterator: yields `&T` in insertion order.
impl<'a, T, S> IntoIterator for &'a LinkedHashSet<T, S> {
    type Item = &'a T;
    type IntoIter = SetIter<'a, T>;

    #[inline]
    fn into_iter(self) -> SetIter<'a, T> {
        self.iter()
    }
}

/// Iterator over `&T` elements in insertion order.
///
/// Created by [`LinkedHashSet::iter`].
pub struct SetIter<'a, T> {
    inner: Keys<'a, T, ()>,
}

impl<'a, T> Iterator for SetIter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for SetIter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        self.inner.next_back()
    }
}

impl<T> ExactSizeIterator for SetIter<'_, T> {}

/// Draining iterator - removes and yields every element in insertion order,
/// leaving the set empty.
///
/// If the iterator is dropped before it is fully consumed, the remaining
/// elements are freed and the set is left in a valid empty state.
///
/// Created by [`LinkedHashSet::drain`].
pub struct SetDrain<'a, T> {
    inner: MapDrain<'a, T, ()>,
}

impl<T> Iterator for SetDrain<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for SetDrain<'_, T> {}

/// Consuming iterator - yields every element in insertion order.
///
/// Created by calling `.into_iter()` on a [`LinkedHashSet`].
pub struct SetIntoIter<T> {
    inner: MapIntoIter<T, ()>,
}

impl<T> Iterator for SetIntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for SetIntoIter<T> {}

#[cfg(not(coverage))]
const _: () = {
    /// `SetIter<'long, T>` is covariant in `'a` and `T`.
    fn _check_set_iter<'long: 'short, 'short, T>(x: SetIter<'long, T>) -> SetIter<'short, T> {
        x
    }
};
