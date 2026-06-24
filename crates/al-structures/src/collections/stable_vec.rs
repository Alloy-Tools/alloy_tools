/// Generation counter used in [`GenerationKey`].
pub type Generation = u64;

/// A type‑safe, generational index.
///
/// Each key is a combination of an `index` and a `generation`.
/// The `StableVec` guarantees that a key is only valid if both
/// the index and the generation match the current state.
/// This prevents use‑after‑free bugs even when indices are
/// recycled.
#[cfg_attr(
    any(test, feature = "serde"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationKey {
    index: usize,
    generation: Generation,
}

impl GenerationKey {
    pub fn new(index: usize, generation: Generation) -> Self {
        Self { index, generation }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn generation(&self) -> Generation {
        self.generation
    }
}

//QUESTION: Maybe make a StableSlice for stack allocated slice. Then add as an `IndexedStorage`.
/// A stable, generational vector with O(1) insertion, deletion, and access.
/// Inspired by the video [Magic container by Pezzza's Work](https://www.youtube.com/watch?v=L4xOCvELWlU)
///
/// # Design
/// - Public indices ([`GenerationKey`]) remain stable until the element is removed.
/// - Removal is O(1) via swap‑and‑pop with the last element.
/// - Iteration is over the contiguous `data` vector with no gaps, cache‑friendly.
/// - Memory efficient: no holes, no separate allocations per element
///
/// # Generational indices
/// Every key carries a generation counter. Removing an element increments the
/// generation for that index, so a stale key can never accidentally access a newly
/// inserted element. Re‑insertion reuses the same index but with an increased generation.
///
/// # Internal Structure & invariants
/// - `data`: actual items, always packed (no holes)
/// - `data_index[i]`: maps `GenerationKey.index` `i` to its position in `data`.
///     For a valid key, `data_index[i] < data.len()`.
/// - `id_map[pos]`: the inverse mapping – for position `pos` in `data`, gives the
///   `GenerationKey.index` of the element stored there.
/// - `generations[i]`: the generation that must match the `GenerationKey` for index `i`.
///
/// Invariants:
/// - `data.len() <= data_index.len() == id_map.len() == generations.len()`
/// - For every valid index `i`, `data_index[i]` points to an element and
///   `id_map[data_index[i]] == i`.
/// - After a removal, the generation of the removed index is incremented,
///   making all previously issued keys for that index invalid.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "T: serde::Serialize",
        deserialize = "T: serde::Deserialize<'de>"
    ))
)]
pub struct StableVec<T> {
    data: Vec<T>,                 // Actual stored items (contiguous)
    data_index: Vec<usize>,       // GenerationKey.index -> position in data
    id_map: Vec<usize>,           // position in data -> GenerationKey.index
    generations: Vec<Generation>, // GenerationKey.index -> current generation
}

impl<T> StableVec<T> {
    /// Create an empty `StableVec`.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            data_index: Vec::new(),
            id_map: Vec::new(),
            generations: Vec::new(),
        }
    }

    /// Creates an empty `StableVec` with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            data_index: Vec::with_capacity(capacity),
            id_map: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
        }
    }

    /// Returns the total number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
        self.data_index.reserve(additional);
        self.id_map.reserve(additional);
        self.generations.reserve(additional);
    }

    /// Shrinks the capacity of all internal vectors as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
        self.data_index.shrink_to_fit();
        self.id_map.shrink_to_fit();
        self.generations.shrink_to_fit();
    }

    /// Returns `true` if `key` corresponds to a currently live element.
    ///
    /// A key is valid only if its index is within bounds, its generation matches
    /// the current generation for that index, and the element is still in `data`.
    pub fn is_valid(&self, key: GenerationKey) -> bool {
        key.index < self.generations.len()
            && self.generations[key.index] == key.generation
            && self.data_index[key.index] < self.data.len()
    }

    pub fn contains(&self, key: GenerationKey) -> bool {
        self.is_valid(key)
    }

    /// Inserts `item` and returns a stable [`GenerationKey`] for it.
    ///
    /// If a previously removed slot is available, it will be reused (with an
    /// updated generation). Otherwise a new index is allocated.
    pub fn insert(&mut self, item: T) -> GenerationKey {
        let position = self.data.len();
        if self.id_map.len() == position {
            // No free slot
            self.data.push(item);
            self.data_index.push(position);
            self.id_map.push(position);
            self.generations.push(1); // generation 0 means it never existed
            GenerationKey::new(position, 1)
        } else {
            // reuse a slot
            let reused = self.id_map[position];
            self.data.push(item);
            self.data_index[reused] = position;

            // The generation was already incremented when the slot was previously removed,
            // so the existing generation is valid for this new occupant.
            GenerationKey::new(reused, self.generations[reused])
        }
    }

    /// Removes the element identified by `key`, returning it if it was present.
    ///
    /// After removal the key is invalidated (its generation is bumped).
    /// The element is swapped with the last element to keep the array packed.
    pub fn remove(&mut self, key: GenerationKey) -> Option<T> {
        // Validate public_id might exist
        if !self.is_valid(key) {
            return None;
        }

        let position = self.data_index[key.index];
        let last_position = self.data.len() - 1;

        // Invalidate key immediately
        self.generations[key.index] += 1;

        // If removing the last element, just pop
        if position == last_position {
            return Some(self.data.pop().unwrap());
        }

        let last_index = self.id_map[last_position];

        // Swap elements in data and id_map
        self.data.swap(position, last_position);
        self.id_map.swap(position, last_position);

        // Update data_index after swap
        self.data_index[last_index] = position;
        self.data_index[key.index] = last_position;

        Some(self.data.pop().unwrap())
    }

    /// Keeps only the elements for which `predicate` returns `true`.
    /// Each removed element is taken out in O(1) using swap‑and‑pop.
    pub fn retain(&mut self, mut predicate: impl FnMut(&T) -> bool) {
        let mut i = 0;
        while i < self.data.len() {
            if predicate(&self.data[i]) {
                i += 1;
            } else {
                self.remove(GenerationKey::new(
                    self.id_map[i],
                    self.generations[self.id_map[i]],
                ));
                // After removal, a new element may occupy position i, so don't increment i.
            }
        }
    }

    /// Returns a reference to the element identified by `key`, or `None`.
    pub fn get(&self, key: GenerationKey) -> Option<&T> {
        if self.is_valid(key) {
            Some(&self.data[self.data_index[key.index]])
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element identified by `key`, or `None`.
    pub fn get_mut(&mut self, key: GenerationKey) -> Option<&mut T> {
        if self.is_valid(key) {
            Some(&mut self.data[self.data_index[key.index]])
        } else {
            None
        }
    }

    /// Iterates over all live elements (order is by data position, which
    /// may not match insertion order after removals).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Mutable iterator over all live elements.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    /// Returns an iterator over all live keys.
    ///
    /// The keys are returned in data position order (not necessarily insertion order).
    pub fn keys(&self) -> impl Iterator<Item = GenerationKey> + '_ {
        (0..self.data.len()).map(move |pos| {
            let index = self.id_map[pos];
            GenerationKey::new(index, self.generations[index])
        })
    }

    /// Returns the number of live elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if there are no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes all elements, invalidating every existing key.
    pub fn clear(&mut self) {
        self.data.clear();
        self.data_index.clear();
        self.id_map.clear();
        self.generations.clear();
    }

    /// Compact the stable vector by re-indexing all live elements
    /// to use consecutive indices starting at 0.
    ///
    /// All existing keys are invalidated. Returns a mapping
    /// `map[old_index] = Some(new_key)` for each previously
    /// live element, or `None` if the old index was already dead.
    ///
    /// After this call, the length of the internal metadata
    /// vectors equals the current number of live elements.
    pub fn compact(&mut self) -> Vec<Option<GenerationKey>> {
        let live_count = self.data.len();

        let mut new_data_index = vec![0; live_count];
        let mut new_id_map = vec![0; live_count];
        let mut new_generations = vec![0; live_count];

        let mut remapping = vec![None; self.data_index.len()];

        // For each currently live element (position in `data`),
        // assign a new index and record the mapping.
        for (new_pos, old_key_idx) in self.id_map.iter().take(live_count).copied().enumerate() {
            // Ensure monotonicity: the new generation is strictly greater
            // than any generation that ever existed at this new index.
            let gen = self.generations[new_pos] + 1;
            let new_key = GenerationKey::new(new_pos, gen);

            new_data_index[new_pos] = new_pos;
            new_id_map[new_pos] = new_pos;
            new_generations[new_pos] = gen;

            // Provide the mapping for the old index
            remapping[old_key_idx] = Some(new_key);
        }

        self.data_index = new_data_index;
        self.id_map = new_id_map;
        self.generations = new_generations;

        remapping
    }
}

impl<T> Default for StableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for StableVec<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            data_index: self.data_index.clone(),
            id_map: self.id_map.clone(),
            generations: self.generations.clone(),
        }
    }
}

impl<T> AsRef<[T]> for StableVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.data
    }
}

impl<T: PartialEq> PartialEq for StableVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: Eq> Eq for StableVec<T> {}

impl<T: PartialOrd> PartialOrd for StableVec<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T: Ord> Ord for StableVec<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl<T: std::hash::Hash> std::hash::Hash for StableVec<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<T> std::ops::Index<GenerationKey> for StableVec<T> {
    type Output = T;

    fn index(&self, key: GenerationKey) -> &T {
        self.get(key).expect("invalid key")
    }
}

impl<T> std::ops::IndexMut<GenerationKey> for StableVec<T> {
    fn index_mut(&mut self, key: GenerationKey) -> &mut T {
        self.get_mut(key).expect("invalid key")
    }
}

impl<T> IntoIterator for StableVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<T> FromIterator<T> for StableVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut sv = StableVec::new();
        for item in iter {
            sv.insert(item);
        }
        sv
    }
}

impl<T> Extend<T> for StableVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.insert(item);
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for StableVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: std::fmt::Display> std::fmt::Display for StableVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, item) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, "]")
    }
}

impl<T> From<StableVec<T>> for Vec<T> {
    fn from(sv: StableVec<T>) -> Self {
        sv.data
    }
}

// Custom checked deserialization that validates invariants.
#[cfg(any(test, feature = "serde"))]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for StableVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let unchecked = StableVecUnchecked::deserialize(deserializer)?;
        unchecked.check().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StableVecError {
    /// A live element's generation is 0 (which means "never existed").
    ZeroGeneration {
        /// The key index that has generation 0.
        index: usize,
        /// The position in `data` of the live element that references this index.
        position: usize,
    },
    /// `data_index[id_map[pos]] != pos` for some live position.
    DataIndexMismatch {
        /// The position in `data` where the inconsistency was found.
        position: usize,
        /// The key index (from `id_map[pos]`).
        index: usize,
        /// The expected value of `data_index[index]` (= `pos`).
        expected: usize,
        /// The actual value found in `data_index[index]`.
        actual: usize,
    },
    /// A dead key index (generation > 0, not in use by any live element)
    /// still points into the live data range, violating invariants.
    UndeadElement {
        /// The dead key index.
        dead_index: usize,
        /// The incorrect `data_index[dead_index]` that is < data.len().
        data_index_value: usize,
        /// The current length of the `data` vector (the boundary that was crossed).
        data_len: usize,
    },
    /// The metadata vectors `data_index`, `id_map`, and `generations` have different lengths.
    LengthMismatch {
        /// Length of `data_index`.
        data_index_len: usize,
        /// Length of `id_map`.
        id_map_len: usize,
        /// Length of `generations`.
        generations_len: usize,
    },
    /// An `id_map` entry refers to an index that is out of bounds for the `generations` vector.
    OutOfBounds {
        /// The position in `id_map` (i.e., the data position).
        position: usize,
        /// The id_map value (key index) that was out of bounds.
        id_map_value: usize,
        /// The length of the `generations` vector (maximum allowed index).
        generations_len: usize,
    },
}

impl std::fmt::Display for StableVecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroGeneration { index, position } => write!(f, "live element at data position {position} references key index {index}, which has generation 0"),
            Self::DataIndexMismatch { position, index, expected, actual } => write!(f, "data position {position} (key index {index}) has data_index = {actual} but expected {expected}"),
            Self::UndeadElement { dead_index, data_index_value, data_len } => write!(f, "dead key index {dead_index} points to live element at position {data_index_value} (data length is {data_len})"),
            Self::LengthMismatch {
                data_index_len,
                id_map_len,
                generations_len,
            } => {
                write!(
                    f,
                    "metadata vector length mismatch: data_index={data_index_len}, id_map={id_map_len}, generations={generations_len}",
                )
            }
            Self::OutOfBounds {
                position,
                id_map_value,
                generations_len,
            } => {
                write!(
                    f,
                    "id_map[{position}] = {id_map_value} is out of bounds (generations length = {generations_len})",
                )
            }
        }
    }
}

impl std::error::Error for StableVecError {}

/// Unchecked StableVec creation for serde.
#[cfg_attr(
    any(test, feature = "serde"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(test, feature = "serde"),
    serde(bound(
        serialize = "T: serde::Serialize",
        deserialize = "T: serde::Deserialize<'de>"
    ))
)]
pub struct StableVecUnchecked<T> {
    data: Vec<T>,
    data_index: Vec<usize>,
    id_map: Vec<usize>,
    generations: Vec<Generation>,
}

impl<T> StableVecUnchecked<T> {
    #[allow(unused)]
    pub fn from_iters<
        D: IntoIterator<Item = T>,
        I: IntoIterator<Item = usize>,
        M: IntoIterator<Item = usize>,
        G: IntoIterator<Item = Generation>,
    >(
        data: D,
        data_index: I,
        id_map: M,
        generations: G,
    ) -> Self {
        Self {
            data: Vec::from_iter(data),
            data_index: Vec::from_iter(data_index),
            id_map: Vec::from_iter(id_map),
            generations: Vec::from_iter(generations),
        }
    }

    /// Validates the internal consistency and returns a valid [`StableVec`].
    ///
    /// # Errors
    /// Returns an error string describing the first inconsistency found.
    pub fn check(self) -> Result<StableVec<T>, StableVecError> {
        let data_len = self.data.len();
        let gen_len = self.generations.len();

        if self.data_index.len() != gen_len || self.id_map.len() != gen_len {
            return Err(StableVecError::LengthMismatch {
                data_index_len: self.data_index.len(),
                id_map_len: self.id_map.len(),
                generations_len: gen_len,
            });
        }

        for pos in 0..data_len {
            let idx = self.id_map[pos];
            if idx >= gen_len {
                return Err(StableVecError::OutOfBounds {
                    position: pos,
                    id_map_value: idx,
                    generations_len: gen_len,
                });
            }
            if self.generations[idx] == 0 {
                return Err(StableVecError::ZeroGeneration {
                    index: idx,
                    position: pos,
                });
            }
            if self.data_index[idx] != pos {
                return Err(StableVecError::DataIndexMismatch {
                    position: pos,
                    index: idx,
                    expected: pos,
                    actual: self.data_index[idx],
                });
            }
        }

        let mut live = vec![false; gen_len];
        for &idx in &self.id_map[..data_len] {
            live[idx] = true;
        }
        for i in 0..gen_len {
            if !live[i] && self.generations[i] > 0 && self.data_index[i] < data_len {
                return Err(StableVecError::UndeadElement {
                    dead_index: i,
                    data_index_value: self.data_index[i],
                    data_len,
                });
            }
        }

        Ok(StableVec {
            data: self.data,
            data_index: self.data_index,
            id_map: self.id_map,
            generations: self.generations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");
        let id1 = vec.insert("b");
        let id2 = vec.insert("c");

        assert_eq!(vec.get(id0), Some(&"a"));
        assert_eq!(vec.get(id1), Some(&"b"));
        assert_eq!(vec.get(id2), Some(&"c"));
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn remove_and_swap() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");
        let id1 = vec.insert("b");
        let id2 = vec.insert("c");

        // Remove middle element
        assert_eq!(vec.remove(id1), Some("b"));
        assert_eq!(vec.len(), 2);

        // Old IDs should still work for valid elements
        assert_eq!(vec.get(id0), Some(&"a"));
        assert_eq!(vec.get(id1), None); // Deleted
        assert_eq!(vec.get(id2), Some(&"c")); // Still valid but moved
    }

    #[test]
    fn remove_last() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");
        let id1 = vec.insert("b");
        let id2 = vec.insert("c");

        // Remove last element
        assert_eq!(vec.remove(id2), Some("c"));
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.get(id2), None);
        assert_eq!(vec.get(id0), Some(&"a"));
        assert_eq!(vec.get(id1), Some(&"b"));
    }

    #[test]
    fn remove_first() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");
        let id1 = vec.insert("b");
        let id2 = vec.insert("c");

        // Remove first element
        assert_eq!(vec.remove(id0), Some("a"));
        assert_eq!(vec.len(), 2);

        // "c" should have been swapped to position 0
        assert_eq!(vec.get(id2), Some(&"c"));
        assert_eq!(vec.get(id1), Some(&"b"));
    }

    #[test]
    fn iteration() {
        let mut vec = StableVec::new();
        vec.insert("a");
        vec.insert("b");
        vec.insert("c");

        let collected: Vec<_> = vec.iter().copied().collect();
        assert_eq!(collected, vec!["a", "b", "c"]);
    }

    #[test]
    fn iteration_after_removals() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");
        let id1 = vec.insert("b");
        let id2 = vec.insert("c");
        let id3 = vec.insert("d");

        vec.remove(id1);
        vec.remove(id2);

        let collected: Vec<_> = vec.iter().copied().collect();
        // Should be "a" and "d" only
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&"a"));
        assert!(collected.contains(&"d"));
        assert_eq!(vec.get(id0), Some(&"a"));
        assert_eq!(vec.get(id3), Some(&"d"));
    }

    #[test]
    fn stale_key() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");

        assert!(vec.is_valid(id0));
        vec.remove(id0);
        assert!(!vec.is_valid(id0));
        assert!(vec.get(id0).is_none());
    }

    #[test]
    fn reused_index() {
        let mut vec = StableVec::new();
        let k0 = vec.insert("first");
        vec.remove(k0);

        let k1 = vec.insert("second"); // reuses index 0
        assert_ne!(k0, k1, "generation should differ");
        assert!(vec.get(k0).is_none());
        assert_eq!(vec.get(k1), Some(&"second"));
    }

    #[test]
    fn retain() {
        let mut vec = StableVec::new();
        let ka = vec.insert("a");
        let kb = vec.insert("b");
        let kc = vec.insert("c");
        let kd = vec.insert("d");

        vec.retain(|s| s.starts_with('a') || s.starts_with('d'));
        assert_eq!(vec.len(), 2);
        assert!(vec.get(ka).is_some());
        assert!(vec.get(kd).is_some());
        assert!(vec.get(kb).is_none());
        assert!(vec.get(kc).is_none());
    }

    #[test]
    fn keys_iter() {
        let mut v = StableVec::new();
        let _k0 = v.insert(10);
        let k1 = v.insert(20);
        let _k2 = v.insert(30);
        v.remove(k1);
        let keys: Vec<GenerationKey> = v.keys().collect();
        assert_eq!(keys.len(), 2);
        // Each key must be valid
        for key in &keys {
            assert!(v.contains(*key));
        }
        // The stale key should not appear
        assert!(!keys.contains(&k1));
    }

    #[test]
    fn index_operator() {
        let mut v = StableVec::new();
        let k = v.insert("hello");
        assert_eq!(v[k], "hello");
        v[k] = "world";
        assert_eq!(v[k], "world");
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;
        use serde_json;

        #[test]
        fn unchecked_upgrade() {
            let svu = StableVecUnchecked::from_iters(
                vec!["a".to_string(), "b".to_string()],
                vec![0usize, 1usize],
                vec![0usize, 1usize],
                vec![1u64, 1u64],
            );

            let sv = svu.check().expect("unchecked should validate");
            assert_eq!(sv.len(), 2);
            let keys = sv.keys().collect::<Vec<_>>();
            assert_eq!(keys.len(), 2);
            assert_eq!(sv.get(keys[0]), Some(&"a".to_string()));
            assert_eq!(sv.get(keys[1]), Some(&"b".to_string()));
        }

        #[test]
        fn unchecked_serde() {
            let svu = StableVecUnchecked::from_iters(
                vec!["x".to_string(), "y".to_string()],
                vec![0usize, 1usize],
                vec![0usize, 1usize],
                vec![1u64, 1u64],
            );

            let s = serde_json::to_string(&svu).expect("serialize unchecked");
            let de: StableVecUnchecked<String> =
                serde_json::from_str(&s).expect("deserialize unchecked");
            let sv = de.check().expect("check after deserialize");
            assert_eq!(sv.len(), 2);
            let mut vals = sv.iter().cloned().collect::<Vec<_>>();
            vals.sort();
            assert_eq!(vals, vec!["x".to_string(), "y".to_string()]);
        }

        #[test]
        fn checked_serde() {
            let mut sv = StableVec::new();
            sv.insert("p".to_string());
            sv.insert("q".to_string());

            let s = serde_json::to_string(&sv).expect("serialize checked");
            let de: StableVec<String> = serde_json::from_str(&s).expect("deserialize checked");
            assert_eq!(de.len(), 2);
            let mut vals = de.iter().cloned().collect::<Vec<_>>();
            vals.sort();
            assert_eq!(vals, vec!["p".to_string(), "q".to_string()]);
        }
    }
}
