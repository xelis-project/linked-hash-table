//! [`LinkedHashMap`] struct and all its `impl` blocks.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{BuildHasher, Hash, RandomState};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};
use std::ptr::NonNull;

use hashbrown::HashTable;

use crate::iter::{Drain, IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use crate::node::Node;

/// Computes the hash of `key` using the supplied `hash_builder`.
#[inline]
fn make_hash<Q, S>(hash_builder: &S, key: &Q) -> u64
where
    Q: Hash + ?Sized,
    S: BuildHasher,
{
    hash_builder.hash_one(key)
}

/// A hash map that preserves **insertion order** and exposes a
/// [`VecDeque`]-like API with [`insert_back`], [`insert_front`],
/// [`pop_front`], [`pop_back`], [`front`], and [`back`].
///
/// All the usual [`HashMap`] operations (`get`, `get_mut`, `remove`,
/// `contains_key`, `len`, `is_empty`, `clear`, …) are also available.
///
/// ## Ordering contract
///
/// * [`insert_back`] and [`insert_front`] **preserve the position** of an
///   existing key: only the value is updated in-place.  Use
///   [`move_to_back`] / [`move_to_front`] to explicitly reorder an entry.
pub struct LinkedHashMap<K, V, S = RandomState> {
    /// Sentinel head; `head.next` is the first real node (or `tail` if empty).
    head: NonNull<Node<K, V>>,
    /// Sentinel tail; `tail.prev` is the last real node (or `head` if empty).
    tail: NonNull<Node<K, V>>,
    /// Raw hash table that stores *pointers* to nodes.
    ///
    /// The key is stored only in the node itself; no bitwise copy of `K` is
    /// ever made for the table.  This is why `K` does not need `Clone`/`Copy`.
    table: HashTable<NonNull<Node<K, V>>>,
    /// Hasher builder kept separately from the table so it can be borrowed
    /// independently (required for the `insert` + rehash closure pattern).
    hash_builder: S,
}

/// A view into a single entry in a map, similar to
/// [`std::collections::hash_map::Entry`].
pub enum Entry<'a, K, V, S = RandomState> {
    Occupied(OccupiedEntry<'a, K, V, S>),
    Vacant(VacantEntry<'a, K, V, S>),
}

/// A view into an occupied entry in a [`LinkedHashMap`].
pub struct OccupiedEntry<'a, K, V, S = RandomState> {
    map: &'a mut LinkedHashMap<K, V, S>,
    node_ptr: NonNull<Node<K, V>>,
}

/// A view into a vacant entry in a [`LinkedHashMap`].
pub struct VacantEntry<'a, K, V, S = RandomState> {
    map: &'a mut LinkedHashMap<K, V, S>,
    key: K,
    /// Cached hash of `key` to avoid recomputing it during insertion.
    cached_hash: u64,
}

// SAFETY: Entry views are tied to a unique `&'a mut LinkedHashMap` borrow.
// Moving them across threads is safe when the borrowed map (and carried key
// for vacant entries) can be transferred.
unsafe impl<K: Send, V: Send, S: Send> Send for OccupiedEntry<'_, K, V, S> {}
unsafe impl<K: Send, V: Send, S: Send> Send for VacantEntry<'_, K, V, S> {}
unsafe impl<K: Send, V: Send, S: Send> Send for Entry<'_, K, V, S> {}

// SAFETY: Shared references to entry views only permit shared reads unless
// `&mut self` is held. Sync bounds mirror referenced data/map requirements.
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for OccupiedEntry<'_, K, V, S> {}
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for VacantEntry<'_, K, V, S> {}
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for Entry<'_, K, V, S> {}

impl<'a, K, V, S> Entry<'a, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }

    /// Ensures a value is in the entry by inserting `default` if vacant,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of `default`
    /// if vacant, and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting [`Default::default`]
    /// if vacant, and returns a mutable reference to the value in the entry.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential insertion.
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            Entry::Vacant(e) => Entry::Vacant(e),
        }
    }
}

impl<'a, K, V, S> OccupiedEntry<'a, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Gets a reference to the key in the entry.
    #[inline]
    pub fn key(&self) -> &K {
        // SAFETY: `node_ptr` points to a live node currently indexed by map.
        unsafe { Node::key_ref(self.node_ptr.as_ptr()) }
    }

    /// Gets a reference to the value in the entry.
    #[inline]
    pub fn get(&self) -> &V {
        // SAFETY: `node_ptr` points to a live node currently indexed by map.
        unsafe { Node::value_ref(self.node_ptr.as_ptr()) }
    }

    /// Gets a mutable reference to the value in the entry.
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        // SAFETY: `OccupiedEntry` holds exclusive access to the map.
        unsafe { Node::value_mut(self.node_ptr.as_ptr()) }
    }

    /// Converts the entry into a mutable reference to the value.
    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        // SAFETY: Consuming self preserves exclusive access tied to `'a`.
        unsafe { Node::value_mut(self.node_ptr.as_ptr()) }
    }

    /// Sets the value of the entry and returns the old value.
    #[inline]
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    /// Removes the entry from the map and returns the value.
    #[inline]
    pub fn remove(self) -> V {
        self.remove_entry().1
    }

    /// Removes the entry from the map and returns the key-value pair.
    #[inline]
    pub fn remove_entry(self) -> (K, V) {
        let map = self.map;
        let node = self.node_ptr.as_ptr();
        // SAFETY: `node` is live and belongs to `map`.
        unsafe {
            let hash = make_hash(&map.hash_builder, Node::key_ref(node));
            let bucket = map
                .table
                .find_entry(hash, |ptr| std::ptr::eq(ptr.as_ptr(), node))
                .expect("LinkedHashMap invariant violated: occupied entry missing from table");
            bucket.remove();
            LinkedHashMap::<K, V, S>::unlink(node);
            let k = Node::key_read(node);
            let v = Node::value_read(node);
            let _ = Box::from_raw(node);
            (k, v)
        }
    }
}

impl<'a, K, V, S> VacantEntry<'a, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Gets a reference to the key that would be used when inserting.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Takes ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Inserts the value into the map and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V {
        let map = self.map;
        let hash = self.cached_hash;
        let node_ptr = Node::new(self.key, value);
        // SAFETY: append before tail sentinel.
        unsafe {
            let before_tail = (*map.tail.as_ptr()).prev;
            LinkedHashMap::<K, V, S>::link_after(before_tail, node_ptr.as_ptr());
        }
        let hash_builder = &map.hash_builder;
        map.table.insert_unique(hash, node_ptr, |ptr| {
            let k = unsafe { Node::key_ref(ptr.as_ptr()) };
            make_hash(hash_builder, k)
        });
        // SAFETY: node was just inserted and remains live in the map.
        unsafe { Node::value_mut(node_ptr.as_ptr()) }
    }
}

// SAFETY: LinkedHashMap owns all the nodes it points to.  They are allocated
// by Node::new / Node::sentinel and freed only by the Drop impl or by the
// various remove / pop / drain methods that take &mut self.
// A shared reference (&LinkedHashMap) only gives out shared references to
// node data; a mutable reference (&mut LinkedHashMap) gives exclusive access.
// RawTable<NonNull<…>> is !Send/!Sync because NonNull is, but the same
// reasoning as for Box<Node<K,V>> applies: we have full ownership.
unsafe impl<K: Send, V: Send, S: Send> Send for LinkedHashMap<K, V, S> {}
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for LinkedHashMap<K, V, S> {}

impl<K, V> LinkedHashMap<K, V> {
    /// Creates an empty `LinkedHashMap`.
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    /// Creates an empty `LinkedHashMap` with the specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> LinkedHashMap<K, V, S> {
    /// Creates an empty `LinkedHashMap` using the supplied hasher builder.
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_and_hasher(0, hash_builder)
    }

    /// Creates an empty `LinkedHashMap` with the given capacity and hasher.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        let head = Node::sentinel();
        let tail = Node::sentinel();

        // SAFETY: Both sentinel pointers are freshly allocated and valid.
        // We wire them together so head.next == tail and tail.prev == head,
        // representing an empty list.
        unsafe {
            head.as_ptr().write(Node {
                key: MaybeUninit::uninit(),
                value: MaybeUninit::uninit(),
                prev: std::ptr::null_mut(),
                next: tail.as_ptr(),
            });
            tail.as_ptr().write(Node {
                key: MaybeUninit::uninit(),
                value: MaybeUninit::uninit(),
                prev: head.as_ptr(),
                next: std::ptr::null_mut(),
            });
        }

        Self {
            head,
            tail,
            table: HashTable::with_capacity(capacity),
            hash_builder,
        }
    }

    /// Returns the number of key-value pairs in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if the map contains no key-value pairs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Returns a reference to the map's [`BuildHasher`].
    #[inline]
    pub fn hasher(&self) -> &S {
        &self.hash_builder
    }

    /// Removes `node` from the doubly-linked list without freeing its memory.
    ///
    /// # Safety
    ///
    /// `node` must be a currently-linked, non-sentinel real node.
    #[inline]
    unsafe fn unlink(node: *mut Node<K, V>) {
        // SAFETY: node is non-null, properly aligned, and still wired into the
        // list, so dereferencing prev/next is valid.
        unsafe {
            let prev = (*node).prev;
            let next = (*node).next;
            (*prev).next = next;
            (*next).prev = prev;
        }
    }

    /// Inserts `node` into the doubly-linked list immediately after `prev`.
    ///
    /// # Safety
    ///
    /// Both `node` and `prev` must be valid non-null pointers.  `prev.next`
    /// must also be a valid pointer (at minimum the tail sentinel).
    #[inline]
    unsafe fn link_after(prev: *mut Node<K, V>, node: *mut Node<K, V>) {
        // SAFETY: prev, node, and prev.next are all valid pointers.
        unsafe {
            let next = (*prev).next;
            (*node).prev = prev;
            (*node).next = next;
            (*prev).next = node;
            (*next).prev = node;
        }
    }
}

impl<K, V, S> LinkedHashMap<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    pub fn entry<'a>(&'a mut self, key: K) -> Entry<'a, K, V, S> {
        let hash = make_hash(&self.hash_builder, &key);
        if let Some(&node_ptr) = self
            .table
            .find(hash, |ptr| unsafe { Node::key_ref(ptr.as_ptr()) == &key })
        {
            Entry::Occupied(OccupiedEntry {
                map: self,
                node_ptr,
            })
        } else {
            Entry::Vacant(VacantEntry {
                map: self,
                key,
                cached_hash: hash,
            })
        }
    }

    /// Inserts a key-value pair at the **back** (most-recently-inserted end).
    ///
    /// If the key already exists, the value is replaced **in-place** and the
    /// node's position in the ordering is **preserved** (it is not moved).
    /// Returns the old value in that case.
    ///
    /// To also move the node to the back, call [`move_to_back`] after
    /// insertion.
    ///
    /// [`move_to_back`]: LinkedHashMap::move_to_back
    pub fn insert_back(&mut self, key: K, value: V) -> Option<V> {
        let hash = make_hash(&self.hash_builder, &key);

        // Check whether the key is already present.
        if let Some(node_ptr) = self
            .table
            .find(hash, |ptr| unsafe { Node::key_ref(ptr.as_ptr()) == &key })
            .copied()
        {
            // Key already present: update value in-place, preserve position.
            // `key` is simply dropped here — it is never duplicated.
            //
            // SAFETY: node_ptr is a live node; &mut self ensures no other
            // reference to its value exists simultaneously.
            unsafe {
                let old = std::mem::replace(Node::value_mut(node_ptr.as_ptr()), value);
                return Some(old);
            }
        }

        // New key: allocate a node and append before the tail sentinel.
        // The key lives *only* inside the node — no copy goes into the table.
        let node_ptr = Node::new(key, value);
        unsafe {
            let before_tail = (*self.tail.as_ptr()).prev;
            Self::link_after(before_tail, node_ptr.as_ptr());
        }
        // SAFETY: node_ptr is a valid, fully-initialised node.
        // The hasher closure is called during rehashing to recompute hashes;
        // it reads the key directly from the node, so no key duplication occurs.
        let hash_builder = &self.hash_builder;
        self.table.insert_unique(hash, node_ptr, |ptr| {
            let k = unsafe { Node::key_ref(ptr.as_ptr()) };
            make_hash(hash_builder, k)
        });
        None
    }

    /// Inserts a key-value pair at the **front** (least-recently-inserted end).
    ///
    /// If the key already exists, the value is replaced **in-place** and the
    /// node's position in the ordering is **preserved** (it is not moved).
    /// Returns the old value in that case.
    ///
    /// To also move the node to the front, call [`move_to_front`] after
    /// insertion.
    ///
    /// [`move_to_front`]: LinkedHashMap::move_to_front
    pub fn insert_front(&mut self, key: K, value: V) -> Option<V> {
        let hash = make_hash(&self.hash_builder, &key);

        if let Some(node_ptr) = self
            .table
            .find(hash, |ptr| unsafe { Node::key_ref(ptr.as_ptr()) == &key })
            .copied()
        {
            // Key already present: update in-place, preserve position.
            unsafe {
                let old = std::mem::replace(Node::value_mut(node_ptr.as_ptr()), value);
                return Some(old);
            }
        }

        let node_ptr = Node::new(key, value);
        unsafe {
            Self::link_after(self.head.as_ptr(), node_ptr.as_ptr());
        }
        let hash_builder = &self.hash_builder;
        self.table.insert_unique(hash, node_ptr, |ptr| {
            let k = unsafe { Node::key_ref(ptr.as_ptr()) };
            make_hash(hash_builder, k)
        });
        None
    }

    /// Alias for [`insert_back`]: matches the [`HashMap::insert`] signature.
    ///
    /// [`insert_back`]: LinkedHashMap::insert_back
    /// [`HashMap::insert`]: std::collections::HashMap::insert
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_back(key, value)
    }

    /// Returns a reference to the value associated with `key`, if present.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        self.table
            .find(hash, |ptr| unsafe {
                Node::key_ref(ptr.as_ptr()).borrow() == key
            })
            .map(|ptr| unsafe { Node::value_ref(ptr.as_ptr()) })
    }

    /// Returns a mutable reference to the value associated with `key`, if
    /// present.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        // Obtain a copy of the node pointer (table.get takes &self.table);
        // then use &mut self to justify the exclusive &mut V.
        let ptr = self
            .table
            .find(hash, |ptr| unsafe {
                Node::key_ref(ptr.as_ptr()).borrow() == key
            })
            .copied()?;
        // SAFETY: We hold &mut self; no other reference to this value exists.
        Some(unsafe { Node::value_mut(ptr.as_ptr()) })
    }

    /// Returns `(&key, &value)` for the given key, if present.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        self.table
            .find(hash, |ptr| unsafe {
                Node::key_ref(ptr.as_ptr()).borrow() == key
            })
            .map(|ptr| unsafe {
                let node = ptr.as_ptr();
                (Node::key_ref(node), Node::value_ref(node))
            })
    }

    /// Returns `true` if the map contains a value for `key`.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        self.table
            .find(hash, |ptr| unsafe {
                Node::key_ref(ptr.as_ptr()).borrow() == key
            })
            .is_some()
    }

    /// Returns references to the **front** (oldest inserted) key-value pair,
    /// or `None` if the map is empty.
    pub fn front(&self) -> Option<(&K, &V)> {
        // SAFETY: head is a valid sentinel.  If the list is non-empty,
        // head.next is a live real node.
        unsafe {
            let first = (*self.head.as_ptr()).next;
            if first == self.tail.as_ptr() {
                return None;
            }
            Some((Node::key_ref(first), Node::value_ref(first)))
        }
    }

    /// Returns a mutable reference to the **front** key-value pair, or `None`.
    pub fn front_mut(&mut self) -> Option<(&K, &mut V)> {
        // SAFETY: &mut self guarantees exclusive access; no other reference to
        // the node's value can exist.
        unsafe {
            let first = (*self.head.as_ptr()).next;
            if first == self.tail.as_ptr() {
                return None;
            }
            Some((Node::key_ref(first), Node::value_mut(first)))
        }
    }

    /// Returns references to the **back** (most recently inserted) key-value
    /// pair, or `None` if the map is empty.
    pub fn back(&self) -> Option<(&K, &V)> {
        // SAFETY: symmetric to front().
        unsafe {
            let last = (*self.tail.as_ptr()).prev;
            if last == self.head.as_ptr() {
                return None;
            }
            Some((Node::key_ref(last), Node::value_ref(last)))
        }
    }

    /// Returns a mutable reference to the **back** key-value pair, or `None`.
    pub fn back_mut(&mut self) -> Option<(&K, &mut V)> {
        // SAFETY: &mut self guarantees exclusive access.
        unsafe {
            let last = (*self.tail.as_ptr()).prev;
            if last == self.head.as_ptr() {
                return None;
            }
            Some((Node::key_ref(last), Node::value_mut(last)))
        }
    }

    /// Removes and returns the **front** (oldest) key-value pair, or `None`.
    pub fn pop_front(&mut self) -> Option<(K, V)> {
        // SAFETY: If the list is non-empty, head.next is a valid real node.
        unsafe {
            let first = (*self.head.as_ptr()).next;
            if first == self.tail.as_ptr() {
                return None;
            }
            self.remove_node(first)
        }
    }

    /// Removes and returns the **back** (newest) key-value pair, or `None`.
    pub fn pop_back(&mut self) -> Option<(K, V)> {
        // SAFETY: symmetric to pop_front.
        unsafe {
            let last = (*self.tail.as_ptr()).prev;
            if last == self.head.as_ptr() {
                return None;
            }
            self.remove_node(last)
        }
    }

    /// Removes the entry for `key` and returns the value, if present.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    /// Removes the entry for `key` and returns `(key, value)`, if present.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        let bucket = self
            .table
            .find_entry(hash, |ptr| unsafe {
                Node::key_ref(ptr.as_ptr()).borrow() == key
            })
            .ok()?;
        let (node_ptr, _) = bucket.remove();
        unsafe {
            let node = node_ptr.as_ptr();
            Self::unlink(node);
            let k = Node::key_read(node);
            let v = Node::value_read(node);
            let _ = Box::from_raw(node);
            Some((k, v))
        }
    }

    /// Removes a node identified by raw pointer from both the hash table and
    /// the linked list, frees it, and returns its `(K, V)`.
    ///
    /// Using pointer identity for the table lookup avoids any borrow of the
    /// key field across the `remove` call and is faster than a key comparison.
    ///
    /// # Safety
    ///
    /// `node` must be a live, fully-initialised real node that is currently
    /// linked in the list and indexed in the hash table.
    unsafe fn remove_node(&mut self, node: *mut Node<K, V>) -> Option<(K, V)> {
        unsafe {
            // Compute the hash from the node's own key.
            let hash = make_hash(&self.hash_builder, Node::key_ref(node));
            // Locate the bucket by pointer identity — faster than key equality
            // and sidesteps any lifetime issue with borrowing the key field.
            let bucket = self
                .table
                .find_entry(hash, |ptr| std::ptr::eq(ptr.as_ptr(), node))
                .expect("LinkedHashMap invariant violated: node missing from table");
            bucket.remove();
            Self::unlink(node);
            let k = Node::key_read(node);
            let v = Node::value_read(node);
            let _ = Box::from_raw(node);
            Some((k, v))
        }
    }

    /// Retains only entries for which `f(&key, &mut value)` returns `true`.
    /// Elements are visited in **insertion order** (front → back).
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        // SAFETY: We traverse via raw pointers.  The `next` pointer is saved
        // before any unlink so the traversal remains valid even after removal.
        // We use pointer identity for the table lookup to avoid borrowing the
        // key field across the remove call.
        unsafe {
            let mut cur = (*self.head.as_ptr()).next;
            while cur != self.tail.as_ptr() {
                let next = (*cur).next;
                let k = Node::key_ref(cur);
                let v = Node::value_mut(cur);
                if !f(k, v) {
                    // Compute hash while k is still valid, then find by pointer.
                    let hash = make_hash(&self.hash_builder, k);
                    let b = self
                        .table
                        .find_entry(hash, |ptr| std::ptr::eq(ptr.as_ptr(), cur))
                        .expect(
                            "LinkedHashMap invariant violated: retained node missing from table",
                        );
                    b.remove();
                    Self::unlink(cur);
                    Node::key_drop(cur);
                    Node::value_drop(cur);
                    let _ = Box::from_raw(cur);
                }
                cur = next;
            }
        }
    }

    /// Removes all key-value pairs from the map.
    pub fn clear(&mut self) {
        // SAFETY: We free every real node via Node::drop_real, then restore
        // the head ↔ tail sentinel linkage to represent an empty list.
        unsafe {
            let mut cur = (*self.head.as_ptr()).next;
            while cur != self.tail.as_ptr() {
                let next = (*cur).next;
                Node::drop_real(cur);
                cur = next;
            }
            (*self.head.as_ptr()).next = self.tail.as_ptr();
            (*self.tail.as_ptr()).prev = self.head.as_ptr();
        }
        // Clear the hash table (drops the NonNull<Node> pointers stored in it;
        // since NonNull has no Drop impl this just marks all slots as empty and
        // frees any overflow storage — the nodes themselves were freed above).
        self.table.clear();
    }

    /// Moves the entry for `key` to the **back** of the ordering.
    /// Returns `true` if the key was found, `false` otherwise.
    pub fn move_to_back<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        if let Some(&node_ptr) = self.table.find(hash, |ptr| unsafe {
            Node::key_ref(ptr.as_ptr()).borrow() == key
        }) {
            // SAFETY: node_ptr is a live node owned by this map.
            unsafe {
                let node = node_ptr.as_ptr();
                Self::unlink(node);
                let before_tail = (*self.tail.as_ptr()).prev;
                Self::link_after(before_tail, node);
            }
            true
        } else {
            false
        }
    }

    /// Moves the entry for `key` to the **front** of the ordering.
    /// Returns `true` if the key was found, `false` otherwise.
    pub fn move_to_front<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = make_hash(&self.hash_builder, key);
        if let Some(&node_ptr) = self.table.find(hash, |ptr| unsafe {
            Node::key_ref(ptr.as_ptr()).borrow() == key
        }) {
            // SAFETY: node_ptr is a live node owned by this map.
            unsafe {
                let node = node_ptr.as_ptr();
                Self::unlink(node);
                Self::link_after(self.head.as_ptr(), node);
            }
            true
        } else {
            false
        }
    }

    /// Removes all elements in **insertion order**, returning `(K, V)` pairs
    /// via an iterator.  The map is empty after this call (even if the
    /// iterator is dropped before it is fully consumed).
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        // SAFETY: We clear the hash table first (frees its own storage only;
        // NonNull has no Drop impl so the nodes are NOT freed).  Ownership of
        // the nodes is transferred to the Drain iterator, which frees them one
        // by one and restores sentinel linkage in its Drop impl.
        let front = unsafe { (*self.head.as_ptr()).next };
        let len = self.len();
        self.table.clear();
        Drain {
            front,
            tail_ptr: self.tail.as_ptr(),
            head_ptr: self.head.as_ptr(),
            len,
            _marker: PhantomData,
        }
    }
}

impl<K, V, S> LinkedHashMap<K, V, S> {
    /// Returns an iterator over `(&K, &V)` pairs in **insertion order**.
    pub fn iter(&self) -> Iter<'_, K, V> {
        // SAFETY: head and tail are valid sentinels for the lifetime of self.
        // head.next is either the first real node or the tail sentinel when empty.
        unsafe {
            Iter {
                front: (*self.head.as_ptr()).next,
                back: (*self.tail.as_ptr()).prev,
                len: self.len(),
                _marker: PhantomData,
            }
        }
    }

    /// Returns an iterator over `(&K, &mut V)` pairs in **insertion order**.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        // SAFETY: &mut self guarantees no other reference to the node contents
        // exists.  Each node is visited exactly once, so no two mutable
        // references can alias.
        unsafe {
            IterMut {
                front: (*self.head.as_ptr()).next,
                back: (*self.tail.as_ptr()).prev,
                len: self.len(),
                _marker: PhantomData,
            }
        }
    }

    /// Returns an iterator over **keys** in insertion order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Returns an iterator over **values** in insertion order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Returns a mutable iterator over **values** in insertion order.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.iter_mut(),
        }
    }
}

impl<K, V, S> Drop for LinkedHashMap<K, V, S> {
    fn drop(&mut self) {
        // SAFETY: We iterate every real node and free it via Node::drop_real,
        // then free the two sentinel nodes via Node::drop_sentinel (their
        // key/value are uninitialised and must not be dropped).
        // The RawTable's own Drop then runs and frees the table's internal
        // storage; the NonNull pointers it stored have no Drop impl, so the
        // nodes (already freed above) are not touched again.
        unsafe {
            let mut cur = (*self.head.as_ptr()).next;
            while cur != self.tail.as_ptr() {
                let next = (*cur).next;
                Node::drop_real(cur);
                cur = next;
            }
            Node::drop_sentinel(self.head.as_ptr());
            Node::drop_sentinel(self.tail.as_ptr());
        }
    }
}

impl<K, V, S, Q> Index<&Q> for LinkedHashMap<K, V, S>
where
    K: Hash + Eq + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    type Output = V;

    /// Returns a reference to the value for `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not present in the map.
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("LinkedHashMap: key not found")
    }
}

impl<K, V, S, Q> IndexMut<&Q> for LinkedHashMap<K, V, S>
where
    K: Hash + Eq + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    /// Returns a mutable reference to the value for `key`.
    ///
    /// # Panics
    ///
    /// Panics if `key` is not present in the map.
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).expect("LinkedHashMap: key not found")
    }
}

impl<K, V> Default for LinkedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, S> fmt::Debug for LinkedHashMap<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, S1, S2> PartialEq<LinkedHashMap<K, V, S2>> for LinkedHashMap<K, V, S1>
where
    K: PartialEq,
    V: PartialEq,
{
    /// Two maps are equal only when they contain **the same key-value pairs in
    /// the same order**.
    fn eq(&self, other: &LinkedHashMap<K, V, S2>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .all(|((k1, v1), (k2, v2))| k1 == k2 && v1 == v2)
    }
}

impl<K: PartialEq + Eq, V: Eq, S> Eq for LinkedHashMap<K, V, S> {}

impl<K: Clone + Hash + Eq, V: Clone, S: BuildHasher + Clone> Clone for LinkedHashMap<K, V, S> {
    fn clone(&self) -> Self {
        let mut new_map = Self::with_capacity_and_hasher(self.len(), self.hash_builder.clone());
        for (k, v) in self.iter() {
            new_map.insert_back(k.clone(), v.clone());
        }
        new_map
    }
}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for LinkedHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        let mut map = Self::with_capacity(lower);
        for (k, v) in iter {
            map.insert_back(k, v);
        }
        map
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for LinkedHashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert_back(k, v);
        }
    }
}

/// Consuming iterator: yields all `(K, V)` pairs in insertion order.
impl<K, V, S> IntoIterator for LinkedHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        // SAFETY: We must prevent LinkedHashMap::drop from running (it would
        // free all nodes, causing a double-free in IntoIter::drop).
        // Strategy: extract the table and hash_builder via ptr::read, then
        // mem::forget(self) to skip the drop, then drop both fields manually.
        //   - drop(table): frees the raw table's own storage; the NonNull<Node>
        //     values stored in it have no Drop impl, so nodes are NOT freed.
        //   - drop(hash_builder): runs S's destructor (if any).
        let front = unsafe { (*self.head.as_ptr()).next };
        let len = self.len();
        let head = self.head;
        let tail = self.tail;
        let table = unsafe { std::ptr::read(&self.table) };
        let hash_builder = unsafe { std::ptr::read(&self.hash_builder) };
        std::mem::forget(self);
        drop(table);
        drop(hash_builder);
        IntoIter {
            front,
            tail,
            head,
            len,
        }
    }
}

/// Shared-reference iterator: yields `(&K, &V)` in insertion order.
impl<'a, K, V, S> IntoIterator for &'a LinkedHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

/// Mutable-reference iterator: yields `(&K, &mut V)` in insertion order.
impl<'a, K, V, S> IntoIterator for &'a mut LinkedHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}
