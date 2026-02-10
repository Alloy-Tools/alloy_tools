/// A vector that maintains stable indices even after removals using the swap-and-pop technique.
/// Implemented following the video [Magic container by Pezzza's Work](https://www.youtube.com/watch?v=L4xOCvELWlU)
///
/// # Design
/// - Public indices returned to users never change
/// - Unsubscribe is O(1) via swap-and-pop with the last element
/// - Iteration is O(n) with no gaps
/// - Memory efficient: no holes, no separate allocations per element
///
/// # Internal Structure
/// - `data`: Actual stored items
/// - `data_index`: Maps public_id -> position in data
/// - `id_map`: Maps position in data -> public_id (for swap-and-pop)
pub struct StableVec<T> {
    data: Vec<T>,
    data_index: Vec<usize>, // public_id -> position in data
    id_map: Vec<usize>,     // position in data -> public_id
}

impl<T> StableVec<T> {
    /// Create a new empty stable vector
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            data_index: Vec::new(),
            id_map: Vec::new(),
        }
    }

    /// Insert an item and return its stable public ID
    pub fn insert(&mut self, item: T) -> usize {
        let position = self.data.len();
        if self.id_map.len() <= position {
            self.data.push(item);
            self.data_index.push(position);
            self.id_map.push(position);
            position
        } else {
            // Already allocated
            self.data.push(item);

            self.id_map[position] // 6
        }
    }

    /// Remove an item by its public ID, returning it if it exists
    pub fn remove(&mut self, public_id: usize) -> Option<T> {
        // Validate public_id might exist
        if !self.is_valid(public_id) {
            return None;
        }

        let position = self.data_index[public_id];

        // If removing the last element, just pop
        let last_position = self.data.len() - 1;
        if position == last_position {
            return Some(self.data.pop().unwrap());
        }

        let last_public_id = self.id_map[last_position];

        // Swap data
        self.data.swap(position, last_position);
        self.id_map.swap(position, last_position);

        // Update data_index after swap
        self.data_index[last_public_id] = position;
        self.data_index[public_id] = last_position;

        Some(self.data.pop().unwrap())
    }

    /// Check if a public ID is valid and not deleted
    pub fn is_valid(&self, public_id: usize) -> bool {
        public_id < self.data_index.len() && self.data_index[public_id] < self.data.len()
    }

    /// Get a reference to an item by its public ID
    pub fn get(&self, public_id: usize) -> Option<&T> {
        if self.is_valid(public_id) {
            let position = self.data_index[public_id];
            Some(&self.data[position])
        } else {
            None
        }
    }

    /// Get a mutable reference to an item by its public ID
    pub fn get_mut(&mut self, public_id: usize) -> Option<&mut T> {
        if self.is_valid(public_id) {
            let position = self.data_index[public_id];
            Some(&mut self.data[position])
        } else {
            None
        }
    }

    /// Iterate over all valid items
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Mutably iterate over all valid items
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    /// Get the number of valid items
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.data.clear();
        self.data_index.clear();
        self.id_map.clear();
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
        }
    }
}

impl<T> AsRef<[T]> for StableVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.data
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
    fn invalid_ids() {
        let mut vec = StableVec::new();
        let id0 = vec.insert("a");

        assert!(vec.is_valid(id0));
        vec.remove(id0);
        assert!(!vec.is_valid(id0));
        assert!(vec.get(id0).is_none());
    }
}
