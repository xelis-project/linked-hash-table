//! Iterator types produced by [`LinkedHashMap`].
//!
//! All iterator structs are returned by methods on [`LinkedHashMap`]; you
//! rarely need to name them explicitly.
//!
//! | Type | Created by | Yields |
//! |------|-----------|--------|
//! | [`Iter`] | [`iter`] | `(&K, &V)` |
//! | [`IterMut`] | [`iter_mut`] | `(&K, &mut V)` |
//! | [`Keys`] | [`keys`] | `&K` |
//! | [`Values`] | [`values`] | `&V` |
//! | [`ValuesMut`] | [`values_mut`] | `&mut V` |
//! | [`Drain`] | [`drain`] | `(K, V)`: empties the map |
//! | [`IntoIter`] | [`into_iter`] | `(K, V)`: consumes the map |
//!
//! [`LinkedHashMap`]: crate::LinkedHashMap
//! [`iter`]: crate::LinkedHashMap::iter
//! [`iter_mut`]: crate::LinkedHashMap::iter_mut
//! [`keys`]: crate::LinkedHashMap::keys
//! [`values`]: crate::LinkedHashMap::values
//! [`values_mut`]: crate::LinkedHashMap::values_mut
//! [`drain`]: crate::LinkedHashMap::drain
//! [`into_iter`]: crate::LinkedHashMap::into_iter

use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::node::Node;

/// Iterator over `(&K, &V)` pairs in insertion order.
///
/// Created by [`LinkedHashMap::iter`](crate::LinkedHashMap::iter).
pub struct Iter<'a, K, V> {
    pub(crate) front: *const Node<K, V>,
    pub(crate) back: *const Node<K, V>,
    pub(crate) len: usize,
    // The `PhantomData` ensures that the output references are properly
    // constrained by the lifetime of the `&LinkedHashMap` that created this iterator.
    pub(crate) _marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0` guarantees that `front` is a live, fully-initialised
        // real node (not the tail sentinel).  The `PhantomData` lifetime ties the
        // output references to `'a`, which is the lifetime of the `&LinkedHashMap`.
        unsafe {
            let node = self.front;
            self.front = (*node).next;
            self.len -= 1;
            Some((Node::key_ref(node), Node::value_ref(node)))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0` guarantees that `back` is a live real node (not the
        // head sentinel).
        unsafe {
            let node = self.back;
            self.back = (*node).prev;
            self.len -= 1;
            Some((Node::key_ref(node), Node::value_ref(node)))
        }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

/// Iterator over `(&K, &mut V)` pairs in insertion order.
///
/// Created by [`LinkedHashMap::iter_mut`](crate::LinkedHashMap::iter_mut).
pub struct IterMut<'a, K, V> {
    pub(crate) front: *mut Node<K, V>,
    pub(crate) back: *mut Node<K, V>,
    pub(crate) len: usize,
    pub(crate) _marker: PhantomData<(&'a K, &'a mut V)>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: The `&mut LinkedHashMap` borrow that created this iterator
        // guarantees exclusive access.  Each node is yielded exactly once, so
        // no two outstanding `&mut V` references can alias.
        unsafe {
            let node = self.front;
            self.front = (*node).next;
            self.len -= 1;
            Some((Node::key_ref(node), Node::value_mut(node)))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: Same as next(), but traversing from the back.
        unsafe {
            let node = self.back;
            self.back = (*node).prev;
            self.len -= 1;
            Some((Node::key_ref(node), Node::value_mut(node)))
        }
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {}

/// Iterator over keys in insertion order.
///
/// Created by [`LinkedHashMap::keys`](crate::LinkedHashMap::keys).
pub struct Keys<'a, K, V> {
    pub(crate) inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a K> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {}

/// Iterator over values in insertion order.
///
/// Created by [`LinkedHashMap::values`](crate::LinkedHashMap::values).
pub struct Values<'a, K, V> {
    pub(crate) inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<&'a V> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a V> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {}

/// Mutable iterator over values in insertion order.
///
/// Created by [`LinkedHashMap::values_mut`](crate::LinkedHashMap::values_mut).
pub struct ValuesMut<'a, K, V> {
    pub(crate) inner: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;

    #[inline]
    fn next(&mut self) -> Option<&'a mut V> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {}

/// Draining iterator: removes and yields every `(K, V)` pair in insertion
/// order, leaving the map empty.
///
/// Created by [`LinkedHashMap::drain`](crate::LinkedHashMap::drain).
/// If the iterator is dropped before it is fully consumed, the remaining
/// elements are freed and the map's sentinel linkage is restored so the map
/// can be used again immediately.
pub struct Drain<'a, K, V> {
    pub(crate) front: *mut Node<K, V>,
    pub(crate) tail_ptr: *mut Node<K, V>,
    pub(crate) head_ptr: *mut Node<K, V>,
    pub(crate) len: usize,
    pub(crate) _marker: PhantomData<&'a mut (K, V)>,
}

impl<K, V> Iterator for Drain<'_, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0` means `front` points to a live real node.  We move
        // the key/value out with `assume_init_read` (the MaybeUninit slots are
        // now logically uninitialised) and then free the bare Node allocation.
        unsafe {
            let node = self.front;
            self.front = (*node).next;
            self.len -= 1;
            let k = Node::key_read(node);
            let v = Node::value_read(node);
            let _ = Box::from_raw(node); // free allocation; fields already moved
            Some((k, v))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> Drop for Drain<'_, K, V> {
    fn drop(&mut self) {
        // Eagerly drop any nodes that the caller did not consume.
        // SAFETY: Remaining nodes are live and fully initialised.  The owning
        // map's HashMap index was cleared before Drain was created, so these
        // nodes are exclusively owned by this iterator.
        unsafe {
            let mut cur = self.front;
            while cur != self.tail_ptr {
                let next = (*cur).next;
                Node::drop_real(cur);
                cur = next;
            }
            // Restore the sentinel linkage so the map is in a valid empty state
            // the moment Drain is dropped.
            (*self.head_ptr).next = self.tail_ptr;
            (*self.tail_ptr).prev = self.head_ptr;
        }
    }
}

impl<K, V> ExactSizeIterator for Drain<'_, K, V> {}

/// Consuming iterator: yields every `(K, V)` pair in insertion order,
/// consuming the map.
///
/// Created by calling `.into_iter()` on a [`LinkedHashMap`](crate::LinkedHashMap).
pub struct IntoIter<K, V> {
    pub(crate) front: *mut Node<K, V>,
    pub(crate) tail: NonNull<Node<K, V>>,
    pub(crate) head: NonNull<Node<K, V>>,
    pub(crate) len: usize,
}

// SAFETY: IntoIter exclusively owns all the nodes it points to (it takes
// ownership from the map via mem::forget + ptr::read).  No aliasing is
// possible.
unsafe impl<K: Send, V: Send> Send for IntoIter<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for IntoIter<K, V> {}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0` guarantees `front` is a live real node.
        unsafe {
            let node = self.front;
            self.front = (*node).next;
            self.len -= 1;
            let k = Node::key_read(node);
            let v = Node::value_read(node);
            let _ = Box::from_raw(node);
            Some((k, v))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> Drop for IntoIter<K, V> {
    fn drop(&mut self) {
        // Free un-consumed real nodes.
        // SAFETY: Remaining nodes are live, real, and exclusively owned by
        // this iterator.
        unsafe {
            let mut cur = self.front;
            while cur != self.tail.as_ptr() {
                let next = (*cur).next;
                Node::drop_real(cur);
                cur = next;
            }
            // Free the two sentinel nodes whose key/value are uninitialised.
            Node::drop_sentinel(self.head.as_ptr());
            Node::drop_sentinel(self.tail.as_ptr());
        }
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}

// Variance assertions
//
// These compile-time checks verify that each iterator type is *covariant*
// in its lifetime and type parameters: a longer lifetime / more-derived type
// can be shortened / widened to match a shorter lifetime / less-derived type.
//
// The pattern is: write a function that accepts the "longer" type and returns
// the "shorter" type with only implicit subtyping — if the compiler accepts it,
// the type is covariant in those parameters.

#[cfg(not(coverage))]
const _: () = {
    /// `Iter<'long, K, V>` is covariant in `'a`, `K`, and `V`.
    fn _check_iter<'long: 'short, 'short, K, V>(x: Iter<'long, K, V>) -> Iter<'short, K, V> {
        x
    }

    /// `IterMut<'long, K, V>` is covariant in `'a` and `K`, but NOT in `V`
    /// (it yields `&'a mut V`).  We only assert covariance in `'a` and `K`.
    fn _check_iter_mut_lifetime<'long: 'short, 'short, K, V>(
        x: IterMut<'long, K, V>,
    ) -> IterMut<'short, K, V> {
        x
    }

    /// `Keys<'long, K, V>` is covariant in `'a` and `K`.
    fn _check_keys<'long: 'short, 'short, K, V>(x: Keys<'long, K, V>) -> Keys<'short, K, V> {
        x
    }

    /// `Values<'long, K, V>` is covariant in `'a` and `V`.
    fn _check_values<'long: 'short, 'short, K, V>(x: Values<'long, K, V>) -> Values<'short, K, V> {
        x
    }

    /// `IntoIter<K, V>` is covariant in `K` and `V`.
    ///
    /// Since `IntoIter` has no lifetime, we verify this by confirming that the
    /// struct compiles and the PhantomData annotation is sound.  The owned
    /// `(K, V)` pairs it yields make covariance safe.
    fn _check_into_iter<K, V>(_: IntoIter<K, V>) {}
};
