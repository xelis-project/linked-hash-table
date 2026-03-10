//! Internal doubly-linked-list node.
//!
//! This is an implementation detail; nothing in this module is part of the
//! public API.

use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// A single node in the doubly-linked list that backs [`LinkedHashMap`].
///
/// **Real nodes** (created by [`Node::new`]) have both `key` and `value`
/// fully initialised.
///
/// **Sentinel nodes** (created by [`Node::sentinel`]) leave `key` and `value`
/// as [`MaybeUninit::uninit()`]; those fields must **never** be read on a
/// sentinel node.
///
/// [`LinkedHashMap`]: crate::LinkedHashMap
pub(crate) struct Node<K, V> {
    pub(crate) key: MaybeUninit<K>,
    pub(crate) value: MaybeUninit<V>,
    pub(crate) prev: *mut Node<K, V>,
    pub(crate) next: *mut Node<K, V>,
}

impl<K, V> Node<K, V> {
    /// Allocates a fully-initialised real node on the heap and returns a
    /// `NonNull` pointer to it.
    pub(crate) fn new(key: K, value: V) -> NonNull<Self> {
        let boxed = Box::new(Self {
            key: MaybeUninit::new(key),
            value: MaybeUninit::new(value),
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        });
        // SAFETY: Box::into_raw never returns null.
        unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) }
    }

    /// Allocates a sentinel node whose key/value are intentionally
    /// uninitialised (they are never read).
    pub(crate) fn sentinel() -> NonNull<Self> {
        let boxed = Box::new(Self {
            key: MaybeUninit::uninit(),
            value: MaybeUninit::uninit(),
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        });
        // SAFETY: Box::into_raw never returns null.
        unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) }
    }

    /// Returns a shared reference to the key field.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real (non-sentinel) node whose key is
    /// initialised.
    #[inline]
    pub(crate) unsafe fn key_ref<'a>(ptr: *const Self) -> &'a K {
        unsafe { (*ptr).key.assume_init_ref() }
    }

    /// Returns a shared reference to the value field.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real (non-sentinel) node whose value is
    /// initialised.
    #[inline]
    pub(crate) unsafe fn value_ref<'a>(ptr: *const Self) -> &'a V {
        unsafe { (*ptr).value.assume_init_ref() }
    }

    /// Returns a mutable reference to the value field.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real (non-sentinel) node whose value is
    /// initialised, and the caller must guarantee unique mutable access.
    #[inline]
    pub(crate) unsafe fn value_mut<'a>(ptr: *mut Self) -> &'a mut V {
        unsafe { (*ptr).value.assume_init_mut() }
    }

    /// Moves the key out of the node.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real node with an initialised key. The key must
    /// not be read/dropped again afterwards.
    #[inline]
    pub(crate) unsafe fn key_read(ptr: *mut Self) -> K {
        unsafe { (*ptr).key.assume_init_read() }
    }

    /// Moves the value out of the node.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real node with an initialised value. The value
    /// must not be read/dropped again afterwards.
    #[inline]
    pub(crate) unsafe fn value_read(ptr: *mut Self) -> V {
        unsafe { (*ptr).value.assume_init_read() }
    }

    /// Drops the key in place.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real node with an initialised key, and key drop
    /// must happen at most once.
    #[inline]
    pub(crate) unsafe fn key_drop(ptr: *mut Self) {
        unsafe { (*ptr).key.assume_init_drop() }
    }

    /// Drops the value in place.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a real node with an initialised value, and value
    /// drop must happen at most once.
    #[inline]
    pub(crate) unsafe fn value_drop(ptr: *mut Self) {
        unsafe { (*ptr).value.assume_init_drop() }
    }

    /// Drops the key and value, then frees the heap allocation.
    ///
    /// # Safety
    ///
    /// - `ptr` was created by [`Node::new`] (both fields are fully
    ///   initialised).
    /// - `ptr` is no longer reachable from the linked list or hash map.
    /// - Called at most once per allocation.
    pub(crate) unsafe fn drop_real(ptr: *mut Self) {
        // SAFETY: `MaybeUninit` does not auto-drop its contents; we explicitly
        // drop K and V before reclaiming the Box to avoid memory leaks.
        // After the drops, both fields are logically uninitialised, so the
        // subsequent Box::from_raw is safe.
        unsafe {
            Self::key_drop(ptr);
            Self::value_drop(ptr);
            let _ = Box::from_raw(ptr);
        }
    }

    /// Frees the heap allocation of a sentinel node **without** touching its
    /// uninitialised key/value fields.
    ///
    /// # Safety
    ///
    /// `ptr` was created by [`Node::sentinel`].
    pub(crate) unsafe fn drop_sentinel(ptr: *mut Self) {
        // SAFETY: Reconstructs the Box from the same pointer returned by
        // Box::into_raw in Node::sentinel.  MaybeUninit<K> and MaybeUninit<V>
        // have no-op Drop impls, so the Box destructor does not touch the
        // uninitialised data.
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
