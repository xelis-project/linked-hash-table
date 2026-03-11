//! # LinkedHashMap
//!
//! A hash map that maintains **insertion order** and supports efficient
//! O(1) push/pop from both the **front** and the **back**, similar to `VecDeque`.
//!
//! ## Design
//!
//! Each key-value pair lives in a heap-allocated node. The nodes form a
//! doubly-linked list that tracks insertion order. A `HashMap<K, NonNull<Node<K, V>>>`
//! gives O(1) lookup by key. Because all node pointers are owned by the map and the
//! `HashMap` only stores raw pointers (never moves the pointed-to memory), pointer
//! stability is guaranteed as long as the map is alive and the entry is not removed.
//!
//! ```text
//!  head ⟷ node_A ⟷ node_B ⟷ node_C ⟷ tail   (sentinel nodes)
//!            ↑           ↑           ↑
//!        HashMap entries (key -> *mut Node)
//! ```
//!
//! The `head` and `tail` fields are **sentinel** nodes that never hold real data.
//! Their `key` / `value` are never read; they merely anchor the list so that
//! every "real" node always has both a `prev` and a `next`, eliminating many
//! `Option` branches in the hot path.
//!
//! ## Ordering contract
//!
//! [`LinkedHashMap::insert_back`] and [`LinkedHashMap::insert_front`] **preserve
//! the position** of an existing key: only the value is updated in-place. Use
//! [`LinkedHashMap::move_to_back`] / [`LinkedHashMap::move_to_front`] to
//! explicitly reorder an entry.

pub mod iter;
mod map;
pub(crate) mod node;
mod set;

pub use iter::{Drain, IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
pub use map::{Entry, LinkedHashMap, OccupiedEntry, VacantEntry};
pub use set::{LinkedHashSet, SetDrain, SetIntoIter, SetIter};

#[cfg(test)]
mod tests;
