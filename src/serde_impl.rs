//! [`serde::Serialize`] and [`serde::Deserialize`] implementations for
//! [`LinkedHashMap`] and [`LinkedHashSet`].
//!
//! Enabled with the `serde` Cargo feature.

use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{LinkedHashMap, LinkedHashSet};

/// Serializes the map as an ordered sequence of key-value pairs.
/// Insertion order is preserved.
impl<K, V, S> Serialize for LinkedHashMap<K, V, S>
where
    K: Serialize + Hash + Eq,
    V: Serialize,
    S: BuildHasher,
{
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

struct LinkedHashMapVisitor<K, V, S> {
    _marker: PhantomData<(K, V, S)>,
}

impl<'de, K, V, S> Visitor<'de> for LinkedHashMapVisitor<K, V, S>
where
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    type Value = LinkedHashMap<K, V, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut map =
            LinkedHashMap::with_capacity_and_hasher(access.size_hint().unwrap_or(0), S::default());
        while let Some((k, v)) = access.next_entry()? {
            map.insert_back(k, v);
        }
        Ok(map)
    }
}

/// Deserializes the map from an ordered sequence of key-value pairs.
/// Insertion order matches the order of entries in the source.
impl<'de, K, V, S> Deserialize<'de> for LinkedHashMap<K, V, S>
where
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(LinkedHashMapVisitor {
            _marker: PhantomData,
        })
    }
}

/// Serializes the set as an ordered sequence. Insertion order is preserved.
impl<T, S> Serialize for LinkedHashSet<T, S>
where
    T: Serialize + Hash + Eq,
    S: BuildHasher,
{
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for item in self {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

struct LinkedHashSetVisitor<T, S> {
    _marker: PhantomData<(T, S)>,
}

impl<'de, T, S> Visitor<'de> for LinkedHashSetVisitor<T, S>
where
    T: Deserialize<'de> + Hash + Eq,
    S: BuildHasher + Default,
{
    type Value = LinkedHashSet<T, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut set =
            LinkedHashSet::with_capacity_and_hasher(access.size_hint().unwrap_or(0), S::default());
        while let Some(item) = access.next_element()? {
            set.insert_back(item);
        }
        Ok(set)
    }
}

/// Deserializes the set from a sequence. Insertion order matches the source.
impl<'de, T, S> Deserialize<'de> for LinkedHashSet<T, S>
where
    T: Deserialize<'de> + Hash + Eq,
    S: BuildHasher + Default,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(LinkedHashSetVisitor {
            _marker: PhantomData,
        })
    }
}
