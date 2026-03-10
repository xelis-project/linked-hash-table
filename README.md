# Linked Hash Table

A Rust `LinkedHashMap` / `LinkedHashSet` implementation that preserves insertion order while keeping fast hash-based lookups.

## Features

- Insertion-order iteration
- O(1)-style lookup by key
- Push/pop at both ends (`front` / `back`)
- `LinkedHashSet` built on top of `LinkedHashMap`
- No reordering happening on already existing keys

## Data structure

The map combines:

- A hash table index (`hashbrown 0.16`) for key lookup
- A doubly-linked list for deterministic order
- Sentinel `head` / `tail` nodes to simplify edge cases

This enables stable order operations while preserving efficient key access.

## Usage

```rust
use linked_hash_table::{LinkedHashMap, LinkedHashSet};

fn main() {
    let mut map = LinkedHashMap::new();

    map.insert_back("a", 1);
    map.insert_back("b", 2);
    map.insert_front("z", 0);

    assert_eq!(map.front(), Some((&"z", &0)));
    assert_eq!(map.back(), Some((&"b", &2)));

    // Entry API
    *map.entry("b").or_insert(10) += 1;
    assert_eq!(map.get("b"), Some(&3));

    // Reordering
    map.move_to_front(&"b");
    assert_eq!(map.front(), Some((&"b", &3)));

    // Set API
    let mut set = LinkedHashSet::new();
    set.insert_back("x");
    set.insert_back("y");
    assert!(set.contains(&"x"));
}
```

## Development

Run tests:

```bash
cargo test
```

Run Miri (strict provenance):

```bash
RUSTUP_TOOLCHAIN="nightly" MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test
```

Run coverage:

```bash
cargo llvm-cov --summary-only
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.