use core::panic;
use std::num::NonZeroU32;

/// A Timer Entry stored in the slab allocator
#[derive(Debug)]
pub struct TimerEntry<T> {
    pub task: T,
    pub deadline: u64,
    // 4 byte indices instead of 16-byte Option<usize>
    pub next: Option<NonZeroU32>, // Index of the next TimerEntry in the slab
    pub prev: Option<NonZeroU32>, // Index of the previous TimerEntry in the slab
    pub level: u8,                // Changed from usize to u8 for efficiency
}

enum Entry<T> {
    Occupied(TimerEntry<T>),
    Free(Option<NonZeroU32>), // Points to the next free entry
}

/// Slab Allocator for cache friendly memory layout
pub struct Slab<T> {
    entries: Vec<Entry<T>>,
    next_free: Option<NonZeroU32>,
}

impl<T> Slab<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(1024), // Preallocate some space
            next_free: None,                   // No free entries initially
        }
    }

    /// Allocate a new entry, resusing freed slots if available
    pub fn alloc(&mut self, task: T, deadline: u64, level: u8) -> NonZeroU32 {
        let entry = TimerEntry {
            task,
            deadline,
            next: None,
            prev: None,
            level,
        };

        if let Some(idx) = self.next_free {
            // Reuse a free slot
            // Convert 1-based NonZeroU32 to 0-based usize
            let vec_idx = (idx.get() - 1) as usize;

            match self.entries[vec_idx] {
                Entry::Free(next_idx) => {
                    self.next_free = next_idx;
                }
                _ => panic!("Corrupted free list"),
            }
            self.entries[vec_idx] = Entry::Occupied(entry);
            return idx;
        }
        // push a new slot
        self.entries.push(Entry::Occupied(entry));
        //Get the new length
        let index = self.entries.len();

        // Safety: Vector length is guaranteed to be > 0 here
        unsafe { NonZeroU32::new_unchecked(index as u32) }
    }

    /// Takes a handle (1-based),converts to 0-based index, and frees the entry
    pub fn free(&mut self, handle: NonZeroU32) -> Option<T> {
        let idx = (handle.get() - 1) as usize;

        if idx >= self.entries.len() {
            return None; // Invalid handle
        }

        // 1. Swap the data out (move it to return it)
        // 2. Replace it with Entry::Free(old_head)
        // 3. Update head to point to this index
        let new_state = Entry::Free(self.next_free);
        let old_state = std::mem::replace(&mut self.entries[idx], new_state);

        match old_state {
            Entry::Occupied(entry) => {
                self.next_free = Some(handle); // This slot is now the head of free list
                Some(entry.task)
            }
            Entry::Free(_) => {
                // It was already free! Restore the state or panic.

                self.entries[idx] = old_state;
                None
            }
        }
    }

    pub fn get(&self, handle: NonZeroU32) -> Option<&TimerEntry<T>> {
        let idx = (handle.get() - 1) as usize;
        match self.entries.get(idx) {
            Some(Entry::Occupied(entry)) => Some(entry),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, handle: NonZeroU32) -> Option<&mut TimerEntry<T>> {
        let idx = (handle.get() - 1) as usize;
        match self.entries.get_mut(idx) {
            Some(Entry::Occupied(entry)) => Some(entry),
            _ => None,
        }
    }
    // Helper to get data without references (for tick loop)
    pub fn remove_and_get_data(&mut self, handle: NonZeroU32) -> Option<(T, u64)> {
        let idx = (handle.get() - 1) as usize;

        if idx >= self.entries.len() {
            return None; // Invalid handle
        }

        // check if occupied first
        let deadline = match &self.entries[idx] {
            Entry::Occupied(e) => e.deadline,
            _ => return None,
        };
        let task = self.free(handle)?;
        Some((task, deadline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut slab = Slab::new();

        let id_a = slab.alloc("Task A", 100, 0);
        let id_b = slab.alloc("Task B", 200, 0);

        assert_eq!(id_a.get(), 1); // First item should be index 1
        assert_eq!(id_b.get(), 2); // Second item should be index 2

        // Verify data integrity
        let entry_a = slab.get(id_a).unwrap();
        assert_eq!(entry_a.task, "Task A");
        assert_eq!(entry_a.deadline, 100);
    }

    #[test]
    fn test_reuse_slots() {
        // This is the CRITICAL test
        let mut slab = Slab::new();

        let id_1 = slab.alloc(1, 10, 0); // Index 1
        let id_2 = slab.alloc(2, 10, 0); // Index 2
        let id_3 = slab.alloc(3, 10, 0); // Index 3

        // Free the middle one (Index 1)
        let freed_val = slab.free(id_2);
        assert_eq!(freed_val, Some(2));

        // Now allocate a new one. It MUST reuse Index 1.
        let id_4 = slab.alloc(4, 10, 0);

        assert_eq!(id_4.get(), 2, "Slab did not reuse the freed slot!");

        // Allocate another. Should be Index 4
        let id_5 = slab.alloc(5, 10, 0);
        assert_eq!(id_5.get(), 4);
    }

    #[test]
    fn test_double_free_protection() {
        let mut slab = Slab::new();
        let id = slab.alloc("A", 10, 0);

        // Free once
        assert!(slab.free(id).is_some());

        // Free again
        assert!(slab.free(id).is_none());
    }

    #[test]
    fn test_lifecycle() {
        let mut slab = Slab::new();

        // store the handles returned by alloc to use them later
        let mut handles = Vec::new();

        // 1. Fill it up
        for i in 0..100 {
            // Store the NonZeroU32 handle
            handles.push(slab.alloc(i, i as u64, 0));
        }

        // 2. Free all even numbers
        // use the stored handles
        for i in (0..100).step_by(2) {
            slab.free(handles[i]);
        }

        // 3. Allocate 50 new items.
        // These should reuse the freed slots (LIFO order).
        for i in 0..50 {
            slab.alloc(i * 100, 0, 0);
        }

        let entry = slab.get(handles[0]).unwrap();
        assert_eq!(entry.task, 4900); // 49 * 100
    }
}
