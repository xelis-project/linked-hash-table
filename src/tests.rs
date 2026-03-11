use super::*;

#[test]
fn test_send_sync_assertions_for_public_types() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<LinkedHashMap<u64, u64>>();
    assert_send_sync::<LinkedHashSet<u64>>();

    assert_send_sync::<Iter<'static, u64, u64>>();
    assert_send_sync::<Keys<'static, u64, u64>>();
    assert_send_sync::<Values<'static, u64, u64>>();
    assert_send_sync::<IntoIter<u64, u64>>();
    assert_send_sync::<SetIter<'static, u64>>();
    assert_send_sync::<SetIntoIter<u64>>();

    // Mutable iterators / drains should be movable across threads but not shared.
    assert_send::<IterMut<'static, u64, u64>>();
    assert_send::<ValuesMut<'static, u64, u64>>();
    assert_send::<Drain<'static, u64, u64>>();
    assert_send::<SetDrain<'static, u64>>();

    // Entry APIs are views over a mutable map borrow, so they should be Send
    // but not Sync.
    assert_send::<Entry<'static, u64, u64>>();
    assert_send::<OccupiedEntry<'static, u64, u64>>();
    assert_send::<VacantEntry<'static, u64, u64>>();

    // Keep at least one explicit Sync assertion helper use here so both
    // bounds are checked by this test function.
    assert_sync::<LinkedHashMap<u64, u64>>();
}

#[test]
fn test_insert_back_and_get() {
    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();
    assert!(m.is_empty());
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);
    assert_eq!(m.len(), 3);
    assert_eq!(m.get("a"), Some(&1));
    assert_eq!(m.get("b"), Some(&2));
    assert_eq!(m.get("c"), Some(&3));
    assert_eq!(m.get("z"), None);
}

#[test]
fn test_insert_front_and_get() {
    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();
    m.insert_front("a", 1);
    m.insert_front("b", 2);
    m.insert_front("c", 3);
    // front-insertion reverses order: c, b, a
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["c", "b", "a"]);
}

#[test]
fn test_insert_alias() {
    let mut m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    m.insert(1, 10);
    m.insert(2, 20);
    assert_eq!(m[&1], 10);
    assert_eq!(m[&2], 20);
}

#[test]
fn test_insert_alias_existing_key_preserves_position() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);

    let old = m.insert("b", 200);
    assert_eq!(old, Some(2));
    assert_eq!(m.get("b"), Some(&200));

    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_insertion_order_back() {
    let mut m = LinkedHashMap::new();
    for i in 0..10u32 {
        m.insert_back(i, i * 2);
    }
    let collected: Vec<_> = m.iter().map(|(&k, &v)| (k, v)).collect();
    let expected: Vec<_> = (0..10).map(|i| (i, i * 2)).collect();
    assert_eq!(collected, expected);
}

#[test]
fn test_insertion_order_front() {
    let mut m = LinkedHashMap::new();
    for i in 0..10u32 {
        m.insert_front(i, i * 2);
    }
    // insert_front(0), insert_front(1), … insert_front(9) -> order: 9,8,…,0
    let keys: Vec<u32> = m.keys().copied().collect();
    let expected: Vec<u32> = (0..10).rev().collect();
    assert_eq!(keys, expected);
}

#[test]
fn test_insert_back_update_preserves_position() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);
    // Re-inserting an existing key: value updated, position unchanged.
    let old = m.insert_back("a", 99);
    assert_eq!(old, Some(1));
    assert_eq!(m.get("a"), Some(&99));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["a", "b", "c"]); // "a" stays at front
}

#[test]
fn test_insert_front_update_preserves_position() {
    let mut m = LinkedHashMap::new();
    m.insert_front("a", 1);
    m.insert_front("b", 2);
    m.insert_front("c", 3);
    // Current order: c, b, a
    // Re-inserting "b": value updated, position unchanged.
    let old = m.insert_front("b", 42);
    assert_eq!(old, Some(2));
    assert_eq!(m.get("b"), Some(&42));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["c", "b", "a"]); // "b" stays in the middle
}

#[test]
fn test_front_back_empty() {
    let m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    assert_eq!(m.front(), None);
    assert_eq!(m.back(), None);
}

#[test]
fn test_front_mut_back_mut_empty() {
    let mut m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    assert_eq!(m.front_mut(), None);
    assert_eq!(m.back_mut(), None);
}

#[test]
fn test_front_back() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, "one");
    m.insert_back(2, "two");
    m.insert_back(3, "three");
    assert_eq!(m.front(), Some((&1, &"one")));
    assert_eq!(m.back(), Some((&3, &"three")));
}

#[test]
fn test_front_mut_back_mut() {
    let mut m = LinkedHashMap::new();
    m.insert_back("x", 10);
    m.insert_back("y", 20);
    if let Some((_, v)) = m.front_mut() {
        *v = 100;
    }
    if let Some((_, v)) = m.back_mut() {
        *v = 200;
    }
    assert_eq!(m["x"], 100);
    assert_eq!(m["y"], 200);
}

#[test]
fn test_pop_front() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, "a");
    m.insert_back(2, "b");
    m.insert_back(3, "c");
    assert_eq!(m.pop_front(), Some((1, "a")));
    assert_eq!(m.pop_front(), Some((2, "b")));
    assert_eq!(m.pop_front(), Some((3, "c")));
    assert_eq!(m.pop_front(), None);
    assert!(m.is_empty());
}

#[test]
fn test_pop_back() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, "a");
    m.insert_back(2, "b");
    m.insert_back(3, "c");
    assert_eq!(m.pop_back(), Some((3, "c")));
    assert_eq!(m.pop_back(), Some((2, "b")));
    assert_eq!(m.pop_back(), Some((1, "a")));
    assert_eq!(m.pop_back(), None);
    assert!(m.is_empty());
}

#[test]
fn test_pop_front_back_alternating() {
    let mut m = LinkedHashMap::new();
    for i in 0..6i32 {
        m.insert_back(i, i);
    }
    assert_eq!(m.pop_front(), Some((0, 0)));
    assert_eq!(m.pop_back(), Some((5, 5)));
    assert_eq!(m.pop_front(), Some((1, 1)));
    assert_eq!(m.pop_back(), Some((4, 4)));
    assert_eq!(m.len(), 2);
}

#[test]
fn test_remove() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);
    assert_eq!(m.remove("b"), Some(2));
    assert_eq!(m.remove("b"), None);
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["a", "c"]);
}

#[test]
fn test_remove_entry() {
    let mut m = LinkedHashMap::new();
    m.insert_back(10u32, "ten");
    m.insert_back(20u32, "twenty");
    assert_eq!(m.remove_entry(&10), Some((10, "ten")));
    assert_eq!(m.back(), Some((&20, &"twenty")));
    assert_eq!(m.front(), Some((&20, &"twenty")));
    assert!(!m.contains_key(&10));
    assert_eq!(m.len(), 1);

    // remove again to exercise the None branch of remove_entry.
    assert_eq!(m.remove_entry(&10), None);

    // Remove the last remaining entry to exercise the empty-map branch of remove_entry.
    assert_eq!(m.remove_entry(&20), Some((20, "twenty")));
    assert!(m.is_empty());
    assert_eq!(m.front(), None);
    assert_eq!(m.back(), None);
}

#[test]
fn test_get_mut() {
    let mut m = LinkedHashMap::new();
    m.insert_back("k", 0i32);
    *m.get_mut("k").unwrap() += 42;
    assert_eq!(m["k"], 42);
}

#[test]
#[should_panic(expected = "key not found")]
fn test_index_missing_key_panics() {
    let m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    let _ = m[&99];
}

#[test]
fn test_index_mut_updates_value() {
    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();
    m.insert_back("x", 1);
    m[&"x"] += 41;
    assert_eq!(m.get("x"), Some(&42));
}

#[test]
fn test_contains_key() {
    let mut m = LinkedHashMap::new();
    m.insert_back("hello", 1);
    assert!(m.contains_key("hello"));
    assert!(!m.contains_key("world"));
}

#[test]
fn test_entry_or_insert() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);

    *m.entry("a").or_insert(10) += 1;
    *m.entry("b").or_insert(20) += 2;

    assert_eq!(m.get("a"), Some(&2));
    assert_eq!(m.get("b"), Some(&22));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["a", "b"]); // new entry inserted at back
}

#[test]
fn test_entry_or_insert_existing_key_preserves_position() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);

    *m.entry("b").or_insert(999) += 10;
    assert_eq!(m.get("b"), Some(&12));

    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_entry_and_modify_or_default() {
    fn add_seven(v: &mut i32) {
        *v += 7;
    }
    fn modify_or_default<'a>(m: &mut LinkedHashMap<&'a str, i32>, key: &'a str) {
        m.entry(key).and_modify(add_seven).or_default();
    }

    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();
    m.insert_back("x", 5);

    modify_or_default(&mut m, "x");
    modify_or_default(&mut m, "y");

    assert_eq!(m.get("x"), Some(&12));
    assert_eq!(m.get("y"), Some(&0));
}

#[test]
fn test_entry_occupied_remove_entry() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);

    match m.entry("a") {
        Entry::Occupied(e) => {
            assert_eq!(e.key(), &"a");
            assert_eq!(e.remove_entry(), ("a", 1));
        }
        Entry::Vacant(_) => panic!("expected occupied entry"),
    }

    assert_eq!(m.get("a"), None);
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["b"]);
}

#[test]
fn test_entry_or_insert_with_and_key() {
    fn make_seven() -> i32 {
        7
    }
    fn get_or_insert_x(m: &mut LinkedHashMap<&str, i32>) -> i32 {
        *m.entry("x").or_insert_with(make_seven)
    }

    let mut m = LinkedHashMap::new();

    assert_eq!(get_or_insert_x(&mut m), 7);

    assert_eq!(get_or_insert_x(&mut m), 7);

    match m.entry("x") {
        Entry::Occupied(e) => assert_eq!(e.key(), &"x"),
        Entry::Vacant(_) => panic!("expected occupied"),
    }
}

#[test]
fn test_entry_key_on_both_variants_and_vacant_key() {
    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();

    let e = m.entry("vac");
    assert_eq!(e.key(), &"vac");
    if let Entry::Vacant(v) = &e {
        assert_eq!(v.key(), &"vac");
    } else {
        panic!("expected vacant");
    }
    let _ = e.or_insert(1);

    let e2 = m.entry("vac");
    assert_eq!(e2.key(), &"vac");
}

#[test]
fn test_get_mut_none_and_remove_entry_none() {
    let mut m: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    assert_eq!(m.get_mut(&1), None);
    assert_eq!(m.remove_entry(&1), None);
}

#[test]
fn test_entry_or_default_existing_value_kept() {
    let mut m: LinkedHashMap<&str, i32> = LinkedHashMap::new();
    m.insert_back("k", 42);
    let v = m.entry("k").or_default();
    assert_eq!(*v, 42);
}

#[test]
fn test_entry_vacant_into_key() {
    let mut m: LinkedHashMap<String, i32> = LinkedHashMap::new();
    match m.entry("hello".to_string()) {
        Entry::Vacant(v) => {
            let k = v.into_key();
            assert_eq!(k, "hello");
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }
    assert!(m.is_empty());
}

#[test]
fn test_entry_occupied_get_get_mut_insert_remove() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);

    match m.entry("a") {
        Entry::Occupied(mut e) => {
            assert_eq!(e.get(), &1);
            *e.get_mut() = 2;
            assert_eq!(e.insert(3), 2);
            assert_eq!(e.remove(), 3);
        }
        Entry::Vacant(_) => panic!("expected occupied"),
    }

    assert!(!m.contains_key("a"));
}

#[test]
fn test_entry_occupied_into_mut() {
    let mut m = LinkedHashMap::new();
    m.insert_back("x", 10);

    if let Entry::Occupied(e) = m.entry("x") {
        let v = e.into_mut();
        *v += 5;
    } else {
        panic!("expected occupied");
    }

    assert_eq!(m.get("x"), Some(&15));
}

#[test]
fn test_map_with_capacity_and_hasher_and_hasher_access() {
    use std::hash::RandomState;
    let hasher = RandomState::new();
    let mut m: LinkedHashMap<i32, i32, _> = LinkedHashMap::with_capacity_and_hasher(32, hasher);
    assert!(m.capacity() >= 32);
    let _ = m.hasher();
    m.insert_back(1, 1);
    assert_eq!(m.get(&1), Some(&1));
}

#[test]
fn test_set_with_hasher_and_capacity_and_hasher_access() {
    use std::hash::RandomState;
    let hasher = RandomState::new();
    let mut s: LinkedHashSet<i32, _> = LinkedHashSet::with_capacity_and_hasher(16, hasher);
    assert!(s.capacity() >= 16);
    let _ = s.hasher();
    assert!(s.insert_back(7));
    assert!(s.contains(&7));
}

#[test]
fn test_set_with_hasher_constructor_and_ref_into_iter() {
    use std::hash::RandomState;
    let hasher = RandomState::new();
    let mut s: LinkedHashSet<&str, _> = LinkedHashSet::with_hasher(hasher);
    s.insert_back("a");
    s.insert_back("b");
    let v: Vec<_> = (&s).into_iter().copied().collect();
    assert_eq!(v, vec!["a", "b"]);
}

#[test]
fn test_string_borrowed_lookup_paths() {
    let mut m: LinkedHashMap<String, i32> = LinkedHashMap::new();
    m.insert_back("hello".to_string(), 1);
    assert!(m.contains_key("hello"));
    assert_eq!(m.get("hello"), Some(&1));
    assert_eq!(
        m.get_key_value("hello").map(|(k, v)| (k.as_str(), *v)),
        Some(("hello", 1))
    );
    assert_eq!(m.remove("hello"), Some(1));
    assert!(m.is_empty());
}

#[test]
fn test_into_iter_partial_drop_path() {
    let mut m = LinkedHashMap::new();
    for i in 0..5 {
        m.insert_back(i, i * 10);
    }
    let mut it = m.into_iter();
    assert_eq!(it.next(), Some((0, 0)));
    assert_eq!(it.next(), Some((1, 10)));
    // Drop with remaining elements to exercise IntoIter::drop cleanup path.
}

#[test]
fn test_keys_values_size_hint_and_double_ended() {
    let mut m = LinkedHashMap::new();
    m.insert_back('a', 1);
    m.insert_back('b', 2);
    m.insert_back('c', 3);

    let mut k = m.keys();
    assert_eq!(k.size_hint(), (3, Some(3)));
    assert_eq!(k.next_back(), Some(&'c'));
    assert_eq!(k.size_hint(), (2, Some(2)));

    let mut v = m.values();
    assert_eq!(v.size_hint(), (3, Some(3)));
    assert_eq!(v.next_back(), Some(&3));
    assert_eq!(v.size_hint(), (2, Some(2)));
}

#[test]
fn test_move_to_back() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);
    assert!(m.move_to_back("a"));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["b", "c", "a"]);
    assert!(!m.move_to_back("z"));
}

#[test]
fn test_move_to_front() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    m.insert_back("c", 3);
    assert!(m.move_to_front("c"));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["c", "a", "b"]);
    assert!(!m.move_to_front("missing"));
}

#[test]
fn test_iter_double_ended() {
    let mut m = LinkedHashMap::new();
    for i in 0..5i32 {
        m.insert_back(i, i * 10);
    }
    let mut it = m.iter();
    assert_eq!(it.next(), Some((&0, &0)));
    assert_eq!(it.next_back(), Some((&4, &40)));
    assert_eq!(it.next(), Some((&1, &10)));
    assert_eq!(it.next_back(), Some((&3, &30)));
    assert_eq!(it.next(), Some((&2, &20)));
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
}

#[test]
fn test_iter_mut() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1i32);
    m.insert_back("b", 2i32);
    for (_, v) in m.iter_mut() {
        *v *= 10;
    }
    assert_eq!(m["a"], 10);
    assert_eq!(m["b"], 20);
}

#[test]
fn test_iter_mut_next_back_and_size_hint() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, 10);
    m.insert_back(2, 20);
    m.insert_back(3, 30);

    let mut it = m.iter_mut();
    assert_eq!(it.size_hint(), (3, Some(3)));
    assert_eq!(it.next_back().map(|(k, v)| (*k, *v)), Some((3, 30)));
    assert_eq!(it.size_hint(), (2, Some(2)));

    let mut empty = LinkedHashMap::<i32, i32>::new();
    let mut it2 = empty.iter_mut();
    assert_eq!(it2.next_back(), None);
}

#[test]
fn test_keys_values_iterators() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1u32, "one");
    m.insert_back(2u32, "two");
    m.insert_back(3u32, "three");
    let keys: Vec<_> = m.keys().copied().collect();
    let values: Vec<_> = m.values().copied().collect();
    assert_eq!(keys, [1, 2, 3]);
    assert_eq!(values, ["one", "two", "three"]);
}

#[test]
fn test_values_mut() {
    let mut m = LinkedHashMap::new();
    m.insert_back("x", 1i32);
    m.insert_back("y", 2i32);
    for v in m.values_mut() {
        *v += 100;
    }
    assert_eq!(m["x"], 101);
    assert_eq!(m["y"], 102);
}

#[test]
fn test_values_mut_size_hint() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, 11);
    m.insert_back(2, 22);
    let it = m.values_mut();
    assert_eq!(it.size_hint(), (2, Some(2)));
}

#[test]
fn test_drain() {
    let mut m = LinkedHashMap::new();
    for i in 0..5i32 {
        m.insert_back(i, i);
    }
    let drained: Vec<_> = m.drain().collect();
    assert_eq!(drained, vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]);
    assert!(m.is_empty());
}

#[test]
fn test_into_iterator_for_refs() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, 10);
    m.insert_back(2, 20);

    let from_ref: Vec<_> = (&m).into_iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(from_ref, vec![(1, 10), (2, 20)]);

    for (_, v) in &mut m {
        *v += 1;
    }
    assert_eq!(m.get(&1), Some(&11));
    assert_eq!(m.get(&2), Some(&21));
}

#[test]
fn test_default_map_and_set() {
    let mut m: LinkedHashMap<i32, i32> = Default::default();
    assert!(m.is_empty());
    m.insert_back(1, 2);
    assert_eq!(m.get(&1), Some(&2));

    let mut s: LinkedHashSet<i32> = Default::default();
    assert!(s.is_empty());
    s.insert_back(7);
    assert!(s.contains(&7));
}

#[test]
fn test_partial_eq_len_mismatch_map_set() {
    let mut a = LinkedHashMap::new();
    a.insert_back(1, 1);
    let mut b = LinkedHashMap::new();
    b.insert_back(1, 1);
    b.insert_back(2, 2);
    assert_ne!(a, b);

    let mut sa = LinkedHashSet::new();
    sa.insert_back(1);
    let mut sb = LinkedHashSet::new();
    sb.insert_back(1);
    sb.insert_back(2);
    assert_ne!(sa, sb);
}

#[test]
fn test_entry_vacant_insert_rehash_path() {
    // Stress the vacant-entry insertion path so HashTable rehashing is likely
    // to occur and the rehash closure is exercised.
    let mut m: LinkedHashMap<i32, i32> = LinkedHashMap::with_capacity(1);
    for i in 0..256 {
        *m.entry(i).or_insert(i * 10) += 1;
    }
    // Second pass over the same keys exercises the occupied-entry branch of
    // the same or_insert monomorphization.
    for i in 0..256 {
        *m.entry(i).or_insert(i * 10) += 1;
    }
    assert_eq!(m.len(), 256);
    assert_eq!(m.get(&0), Some(&2));
    assert_eq!(m.get(&255), Some(&(2550 + 2)));
}

#[test]
fn test_drain_partial() {
    let mut m = LinkedHashMap::new();
    for i in 0..5i32 {
        m.insert_back(i, i);
    }
    {
        let mut d = m.drain();
        assert_eq!(d.next(), Some((0, 0)));
        // Drop drain with remaining elements: they must be freed without UB.
    }
    assert!(m.is_empty());
}

#[test]
fn test_into_iter() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1i32);
    m.insert_back("b", 2i32);
    m.insert_back("c", 3i32);
    let v: Vec<_> = m.into_iter().collect();
    assert_eq!(v, vec![("a", 1), ("b", 2), ("c", 3)]);
}

#[test]
fn test_retain() {
    let mut m = LinkedHashMap::new();
    for i in 0..10i32 {
        m.insert_back(i, i);
    }
    m.retain(|k, _| k % 2 == 0);
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec![0, 2, 4, 6, 8]);
}

#[test]
fn test_clear() {
    let mut m = LinkedHashMap::new();
    m.insert_back(1, 2);
    m.insert_back(3, 4);
    m.clear();
    assert!(m.is_empty());
    assert_eq!(m.front(), None);
    assert_eq!(m.back(), None);
    // Ensure the map is still usable after clear.
    m.insert_back(5, 6);
    assert_eq!(m.len(), 1);
    assert_eq!(m.front(), Some((&5, &6)));
}

#[test]
fn test_clone() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    let m2 = m.clone();
    assert_eq!(m, m2);
}

#[test]
fn test_partial_eq_order_matters() {
    let mut m1 = LinkedHashMap::new();
    m1.insert_back(1, "a");
    m1.insert_back(2, "b");

    let mut m2 = LinkedHashMap::new();
    m2.insert_back(2, "b");
    m2.insert_back(1, "a");

    // Same pairs, different insertion order -> not equal.
    assert_ne!(m1, m2);
}

#[test]
fn test_partial_eq_value_mismatch_and_len_mismatch_i64() {
    let mut a: LinkedHashMap<i64, i64> = LinkedHashMap::new();
    a.insert_back(1, 10);

    let mut b: LinkedHashMap<i64, i64> = LinkedHashMap::new();
    b.insert_back(1, 10);
    assert_eq!(a, b);

    let mut c: LinkedHashMap<i64, i64> = LinkedHashMap::new();
    c.insert_back(1, 11);
    assert_ne!(a, c);

    b.insert_back(2, 20);
    assert_ne!(a, b);
}

#[test]
fn test_partial_eq_value_mismatch_and_len_mismatch_i32_str() {
    let mut a: LinkedHashMap<i32, &str> = LinkedHashMap::new();
    a.insert_back(1, "a");

    let mut b: LinkedHashMap<i32, &str> = LinkedHashMap::new();
    b.insert_back(1, "a");
    assert_eq!(a, b);

    let mut c: LinkedHashMap<i32, &str> = LinkedHashMap::new();
    c.insert_back(1, "z");
    assert_ne!(a, c);

    b.insert_back(2, "b");
    assert_ne!(a, b);
}

#[test]
fn test_debug_format() {
    let mut m = LinkedHashMap::new();
    m.insert_back("a", 1);
    m.insert_back("b", 2);
    let s = format!("{:?}", m);
    assert_eq!(s, r#"{"a": 1, "b": 2}"#);
}

#[test]
fn test_from_iter() {
    let m: LinkedHashMap<_, _> = vec![(1, "one"), (2, "two"), (3, "three")]
        .into_iter()
        .collect();
    assert_eq!(m.len(), 3);
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, [1, 2, 3]);
}

#[test]
fn test_extend() {
    let mut m = LinkedHashMap::new();
    m.insert_back(0, 0);
    m.extend(vec![(1, 1), (2, 2)]);
    assert_eq!(m.len(), 3);
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, [0, 1, 2]);
}

#[test]
fn test_single_element() {
    let mut m = LinkedHashMap::new();
    m.insert_back(42u32, "hello");
    assert_eq!(m.front(), Some((&42, &"hello")));
    assert_eq!(m.back(), Some((&42, &"hello")));
    assert_eq!(m.pop_front(), Some((42, "hello")));
    assert!(m.is_empty());
}

#[test]
fn test_large_insert_pop() {
    #[cfg(miri)]
    const N: u64 = 100;
    #[cfg(not(miri))]
    const N: u64 = 10_000;

    let mut m = LinkedHashMap::new();
    for i in 0..N {
        m.insert_back(i, i * i);
    }
    for i in 0..N {
        assert_eq!(m.pop_front(), Some((i, i * i)));
    }
    assert!(m.is_empty());
}

#[test]
fn test_insert_front_then_pop_back() {
    let mut m = LinkedHashMap::new();
    for i in 0..5i32 {
        m.insert_front(i, i);
    }
    // Order: 4, 3, 2, 1, 0
    assert_eq!(m.pop_back(), Some((0, 0)));
    assert_eq!(m.pop_back(), Some((1, 1)));
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec![4, 3, 2]);
}

#[test]
fn test_get_key_value() {
    let mut m = LinkedHashMap::new();
    m.insert_back("foo", 99u32);
    assert_eq!(m.get_key_value("foo"), Some((&"foo", &99)));
    assert_eq!(m.get_key_value("bar"), None);
}

#[test]
fn test_mixed_insert_front_back_ordering() {
    let mut m = LinkedHashMap::new();
    m.insert_back("b", 2);
    m.insert_front("a", 1); // a, b
    m.insert_back("c", 3); // a, b, c
    m.insert_front("z", 26); // z, a, b, c
    let keys: Vec<_> = m.keys().copied().collect();
    assert_eq!(keys, vec!["z", "a", "b", "c"]);
}

#[test]
fn test_drop_does_not_leak() {
    // Box<i32> values surface double-frees and leaks in sanitised / Miri runs.
    let mut m = LinkedHashMap::new();
    for i in 0..100 {
        m.insert_back(i, Box::new(i));
    }
    drop(m);
}

#[test]
fn test_exact_size_iterator() {
    let mut m = LinkedHashMap::new();
    for i in 0..7i32 {
        m.insert_back(i, i);
    }
    let mut it = m.iter();
    assert_eq!(it.len(), 7);
    it.next();
    assert_eq!(it.len(), 6);
}

// LinkedHashSet tests

#[test]
fn test_set_insert_back_and_contains() {
    let mut s: LinkedHashSet<&str> = LinkedHashSet::new();
    assert!(s.is_empty());
    assert!(s.insert_back("a"));
    assert!(s.insert_back("b"));
    assert!(s.insert_back("c"));
    assert_eq!(s.len(), 3);
    assert!(s.contains("a"));
    assert!(s.contains("b"));
    assert!(s.contains("c"));
    assert!(!s.contains("z"));
}

#[test]
fn test_set_insert_front_order() {
    let mut s: LinkedHashSet<&str> = LinkedHashSet::new();
    s.insert_front("a");
    s.insert_front("b");
    s.insert_front("c");
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["c", "b", "a"]);
}

#[test]
fn test_set_insert_back_duplicate_is_noop() {
    let mut s = LinkedHashSet::new();
    assert!(s.insert_back("a"));
    assert!(s.insert_back("b"));
    assert!(s.insert_back("c"));
    // Duplicate: returns false, position preserved.
    assert!(!s.insert_back("a"));
    assert_eq!(s.len(), 3);
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["a", "b", "c"]); // "a" stays at front
}

#[test]
fn test_set_insert_front_duplicate_is_noop() {
    let mut s = LinkedHashSet::new();
    s.insert_front("a");
    s.insert_front("b");
    s.insert_front("c");
    // Order: c, b, a. Re-inserting "b": position preserved.
    assert!(!s.insert_front("b"));
    assert_eq!(s.len(), 3);
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["c", "b", "a"]);
}

#[test]
fn test_set_insert_alias() {
    let mut s: LinkedHashSet<i32> = LinkedHashSet::new();
    assert!(s.insert(1));
    assert!(s.insert(2));
    assert!(!s.insert(1)); // duplicate
    assert_eq!(s.len(), 2);

    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec![1, 2]);
}

#[test]
fn test_set_front_back_empty() {
    let s: LinkedHashSet<i32> = LinkedHashSet::new();
    assert_eq!(s.front(), None);
    assert_eq!(s.back(), None);
}

#[test]
fn test_set_front_back() {
    let mut s = LinkedHashSet::new();
    s.insert_back(1);
    s.insert_back(2);
    s.insert_back(3);
    assert_eq!(s.front(), Some(&1));
    assert_eq!(s.back(), Some(&3));
}

#[test]
fn test_set_pop_front() {
    let mut s = LinkedHashSet::new();
    s.insert_back(1);
    s.insert_back(2);
    s.insert_back(3);
    assert_eq!(s.pop_front(), Some(1));
    assert_eq!(s.pop_front(), Some(2));
    assert_eq!(s.pop_front(), Some(3));
    assert_eq!(s.pop_front(), None);
    assert!(s.is_empty());
}

#[test]
fn test_set_pop_back() {
    let mut s = LinkedHashSet::new();
    s.insert_back(1);
    s.insert_back(2);
    s.insert_back(3);
    assert_eq!(s.pop_back(), Some(3));
    assert_eq!(s.pop_back(), Some(2));
    assert_eq!(s.pop_back(), Some(1));
    assert_eq!(s.pop_back(), None);
    assert!(s.is_empty());
}

#[test]
fn test_set_pop_front_back_alternating() {
    let mut s = LinkedHashSet::new();
    for i in 0..6i32 {
        s.insert_back(i);
    }
    assert_eq!(s.pop_front(), Some(0));
    assert_eq!(s.pop_back(), Some(5));
    assert_eq!(s.pop_front(), Some(1));
    assert_eq!(s.pop_back(), Some(4));
    assert_eq!(s.len(), 2);
}

#[test]
fn test_set_remove() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    s.insert_back("c");
    assert!(s.remove("b"));
    assert!(!s.remove("b")); // already removed
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["a", "c"]);
}

#[test]
fn test_set_take() {
    let mut s = LinkedHashSet::new();
    s.insert_back("foo");
    s.insert_back("bar");
    assert_eq!(s.take("foo"), Some("foo"));
    assert_eq!(s.take("foo"), None);
    assert_eq!(s.len(), 1);
}

#[test]
fn test_set_get() {
    let mut s = LinkedHashSet::new();
    s.insert_back("hello");
    assert_eq!(s.get("hello"), Some(&"hello"));
    assert_eq!(s.get("world"), None);
}

#[test]
fn test_set_move_to_back() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    s.insert_back("c");
    assert!(s.move_to_back("a"));
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["b", "c", "a"]);
    assert!(!s.move_to_back("z"));
}

#[test]
fn test_set_move_to_front() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    s.insert_back("c");
    assert!(s.move_to_front("c"));
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["c", "a", "b"]);
}

#[test]
fn test_set_iter_double_ended() {
    let mut s = LinkedHashSet::new();
    for i in 0..5i32 {
        s.insert_back(i);
    }
    let mut it = s.iter();
    assert_eq!(it.next(), Some(&0));
    assert_eq!(it.next_back(), Some(&4));
    assert_eq!(it.next(), Some(&1));
    assert_eq!(it.next_back(), Some(&3));
    assert_eq!(it.next(), Some(&2));
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
}

#[test]
fn test_set_drain() {
    let mut s = LinkedHashSet::new();
    for i in 0..5i32 {
        s.insert_back(i);
    }
    let drained: Vec<_> = s.drain().collect();
    assert_eq!(drained, vec![0, 1, 2, 3, 4]);
    assert!(s.is_empty());
}

#[test]
fn test_set_drain_partial() {
    let mut s = LinkedHashSet::new();
    for i in 0..5i32 {
        s.insert_back(i);
    }
    {
        let mut d = s.drain();
        assert_eq!(d.next(), Some(0));
        // Drop drain early - remaining elements must be freed without UB.
    }
    assert!(s.is_empty());
}

#[test]
fn test_set_into_iter() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    s.insert_back("c");
    let v: Vec<_> = s.into_iter().collect();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn test_set_retain() {
    let mut s = LinkedHashSet::new();
    for i in 0..10i32 {
        s.insert_back(i);
    }
    s.retain(|v| v % 2 == 0);
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec![0, 2, 4, 6, 8]);
}

#[test]
fn test_set_clear() {
    let mut s = LinkedHashSet::new();
    s.insert_back(1);
    s.insert_back(2);
    s.clear();
    assert!(s.is_empty());
    assert_eq!(s.front(), None);
    assert_eq!(s.back(), None);
    // Still usable after clear.
    s.insert_back(3);
    assert_eq!(s.len(), 1);
    assert_eq!(s.front(), Some(&3));
}

#[test]
fn test_set_clone() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    let s2 = s.clone();
    assert_eq!(s, s2);
}

#[test]
fn test_set_partial_eq_order_matters() {
    let mut s1 = LinkedHashSet::new();
    s1.insert_back(1);
    s1.insert_back(2);

    let mut s2 = LinkedHashSet::new();
    s2.insert_back(2);
    s2.insert_back(1);

    // Same elements but with different order: not equal.
    assert_ne!(s1, s2);
}

#[test]
fn test_set_debug_format() {
    let mut s = LinkedHashSet::new();
    s.insert_back("a");
    s.insert_back("b");
    let dbg = format!("{:?}", s);
    assert_eq!(dbg, r#"{"a", "b"}"#);
}

#[test]
fn test_set_from_iter() {
    let s: LinkedHashSet<_> = vec![1, 2, 3].into_iter().collect();
    assert_eq!(s.len(), 3);
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, [1, 2, 3]);
}

#[test]
fn test_set_extend() {
    let mut s = LinkedHashSet::new();
    s.insert_back(0);
    s.extend(vec![1, 2]);
    assert_eq!(s.len(), 3);
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, [0, 1, 2]);
}

#[test]
fn test_set_is_subset() {
    let a: LinkedHashSet<_> = [1, 2].iter().copied().collect();
    let b: LinkedHashSet<_> = [1, 2, 3].iter().copied().collect();
    assert!(a.is_subset(&b));
    assert!(!b.is_subset(&a));
    assert!(a.is_subset(&a));
}

#[test]
fn test_set_is_superset() {
    let a: LinkedHashSet<_> = [1, 2, 3].iter().copied().collect();
    let b: LinkedHashSet<_> = [1, 2].iter().copied().collect();
    assert!(a.is_superset(&b));
    assert!(!b.is_superset(&a));
}

#[test]
fn test_set_is_disjoint() {
    let a: LinkedHashSet<_> = [1, 2, 3].iter().copied().collect();
    let b: LinkedHashSet<_> = [4, 5, 6].iter().copied().collect();
    let c: LinkedHashSet<_> = [3, 4].iter().copied().collect();
    assert!(a.is_disjoint(&b));
    assert!(!a.is_disjoint(&c));
}

#[test]
fn test_set_large_insert_pop() {
    const N: u64 = 10_000;
    let mut s = LinkedHashSet::new();
    for i in 0..N {
        s.insert_back(i);
    }
    for i in 0..N {
        assert_eq!(s.pop_front(), Some(i));
    }
    assert!(s.is_empty());
}

#[test]
fn test_set_drop_does_not_leak() {
    let mut s = LinkedHashSet::new();
    for i in 0..100i32 {
        // Box<i32> to catch double-frees and leaks in sanitised / Miri runs.
        s.insert_back(Box::new(i));
    }
    drop(s);
}

#[test]
fn test_set_exact_size_iterator() {
    let mut s = LinkedHashSet::new();
    for i in 0..7i32 {
        s.insert_back(i);
    }
    let mut it = s.iter();
    assert_eq!(it.len(), 7);
    it.next();
    assert_eq!(it.len(), 6);
}

#[test]
fn test_set_mixed_front_back() {
    let mut s = LinkedHashSet::new();
    s.insert_back("b");
    s.insert_front("a"); // a, b
    s.insert_back("c"); // a, b, c
    s.insert_front("z"); // z, a, b, c
    let elems: Vec<_> = s.iter().copied().collect();
    assert_eq!(elems, vec!["z", "a", "b", "c"]);
}

#[test]
fn test_set_single_element() {
    let mut s = LinkedHashSet::new();
    s.insert_back(42u32);
    assert_eq!(s.front(), Some(&42));
    assert_eq!(s.back(), Some(&42));
    assert_eq!(s.pop_front(), Some(42));
    assert!(s.is_empty());
}

#[test]
fn test_set_insertion_order_front() {
    let mut s = LinkedHashSet::new();
    for i in 0..10u32 {
        s.insert_front(i);
    }

    let elems: Vec<u32> = s.iter().copied().collect();
    let expected: Vec<u32> = (0..10).rev().collect();
    assert_eq!(elems, expected);
}

#[test]
fn test_string_keys_map() {
    let mut m = LinkedHashMap::new();
    m.insert_back(String::from("hello"), 1u32);
    m.insert_back(String::from("world"), 2u32);
    m.insert_back(String::from("hello"), 99u32); // update in-place
    assert_eq!(m.get("hello"), Some(&99u32));
    assert_eq!(m.get("world"), Some(&2u32));
    assert_eq!(m.len(), 2);
    let keys: Vec<&str> = m.keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, ["hello", "world"]);
}

#[test]
fn test_box_keys_map() {
    // Box<i32> as MAP KEYS - formerly caused a double-free.
    let mut m: LinkedHashMap<Box<i32>, &str> = LinkedHashMap::new();
    m.insert_back(Box::new(1), "one");
    m.insert_back(Box::new(2), "two");
    m.insert_back(Box::new(3), "three");
    assert_eq!(m.get(&1i32), Some(&"one"));
    assert_eq!(m.remove(&2i32), Some("two"));
    assert_eq!(m.len(), 2);
}

#[test]
fn test_box_set_elements() {
    // Box<i32> as SET ELEMENTS - formerly caused a double-free when the set
    // bitwise-copied element into its internal index.  Now safe.
    let mut s: LinkedHashSet<Box<i32>> = LinkedHashSet::new();
    for i in 0..50i32 {
        s.insert_back(Box::new(i));
    }
    assert_eq!(s.len(), 50);

    let last = s.back().unwrap();
    {
        let mut cloned = s.clone();
        assert_eq!(cloned.len(), 50);
        cloned.clear();

        assert!(cloned.is_empty());
        assert!(cloned.front().is_none());
        assert!(cloned.back().is_none());
    }

    assert_eq!(**last, 49);
    assert_eq!(s.len(), 50);

    while s.pop_front().is_some() {}
    assert!(s.is_empty());
}

#[test]
fn test_string_set_elements() {
    let mut s = LinkedHashSet::new();
    s.insert_back(String::from("alpha"));
    s.insert_back(String::from("beta"));
    s.insert_back(String::from("gamma"));
    assert!(s.contains("beta"));
    assert_eq!(s.pop_front(), Some(String::from("alpha")));
    let elems: Vec<_> = s.iter().map(|x| x.as_str()).collect();
    assert_eq!(elems, ["beta", "gamma"]);
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn test_map_serialize_preserves_insertion_order() {
        let mut m = LinkedHashMap::new();
        m.insert_back("c", 3);
        m.insert_back("a", 1);
        m.insert_back("b", 2);

        let json = serde_json::to_string(&m).unwrap();
        // JSON object must reflect insertion order: c, a, b
        assert_eq!(json, r#"{"c":3,"a":1,"b":2}"#);
    }

    #[test]
    fn test_map_deserialize_preserves_source_order() {
        let json = r#"{"x":10,"y":20,"z":30}"#;
        let m: LinkedHashMap<String, i32> = serde_json::from_str(json).unwrap();

        let keys: Vec<_> = m.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, ["x", "y", "z"]);
        assert_eq!(m.get("x"), Some(&10));
        assert_eq!(m.get("y"), Some(&20));
        assert_eq!(m.get("z"), Some(&30));
    }

    #[test]
    fn test_map_round_trip() {
        let mut original = LinkedHashMap::new();
        original.insert_back("one", 1_i64);
        original.insert_back("two", 2_i64);
        original.insert_back("three", 3_i64);

        let json = serde_json::to_string(&original).unwrap();
        let restored: LinkedHashMap<String, i64> = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 3);
        let pairs: Vec<_> = restored.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(pairs, [("one", 1), ("two", 2), ("three", 3)]);
    }

    #[test]
    fn test_map_empty_round_trip() {
        let original: LinkedHashMap<String, i32> = LinkedHashMap::new();
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "{}");
        let restored: LinkedHashMap<String, i32> = serde_json::from_str(&json).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_set_serialize_preserves_insertion_order() {
        let mut s = LinkedHashSet::new();
        s.insert_back("c");
        s.insert_back("a");
        s.insert_back("b");

        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"["c","a","b"]"#);
    }

    #[test]
    fn test_set_deserialize_preserves_source_order() {
        let json = r#"[10, 30, 20]"#;
        let s: LinkedHashSet<i32> = serde_json::from_str(json).unwrap();

        let elems: Vec<_> = s.iter().copied().collect();
        assert_eq!(elems, [10, 30, 20]);
    }

    #[test]
    fn test_set_round_trip() {
        let mut original: LinkedHashSet<String> = LinkedHashSet::new();
        original.insert_back(String::from("alpha"));
        original.insert_back(String::from("beta"));
        original.insert_back(String::from("gamma"));

        let json = serde_json::to_string(&original).unwrap();
        let restored: LinkedHashSet<String> = serde_json::from_str(&json).unwrap();

        let elems: Vec<_> = restored.iter().map(|s| s.as_str()).collect();
        assert_eq!(elems, ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_set_empty_round_trip() {
        let original: LinkedHashSet<i32> = LinkedHashSet::new();
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "[]");
        let restored: LinkedHashSet<i32> = serde_json::from_str(&json).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_map_nested_value_round_trip() {
        let mut m: LinkedHashMap<String, Vec<i32>> = LinkedHashMap::new();
        m.insert_back(String::from("odds"), vec![1, 3, 5]);
        m.insert_back(String::from("evens"), vec![2, 4, 6]);

        let json = serde_json::to_string(&m).unwrap();
        let restored: LinkedHashMap<String, Vec<i32>> = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.get("odds"), Some(&vec![1, 3, 5]));
        assert_eq!(restored.get("evens"), Some(&vec![2, 4, 6]));
        let keys: Vec<_> = restored.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, ["odds", "evens"]);
    }
}
