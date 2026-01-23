use core::panic;

const WHEEL_SIZE: usize = 64; //2^6 slots per wheel
const WHEEL_BITS: u32 = 6;
const NUM_LEVELS: usize = 4; //Supports upto 64^4 = 16,777,216 ticks


/// A Timer Entry stored in the slab allocator
#[derive(Debug)]
pub struct TimerEntry<T> {
    pub task: T,
    pub deadline: u64,
    pub next: Option<usize>, // Index of the next TimerEntry in the slab
    pub prev: Option<usize>, // Index of the previous TimerEntry in the slab
}

enum Entry<T> {
    Occupied(TimerEntry<T>),
    Free(usize), // Points to the next free entry
}


/// Slab Allocator for cache friendly memory layout
pub struct Slab<T> {
    entries: Vec<Entry<T>>,
    next_free: usize,
}

impl<T> Slab<T>{
    pub fn new() -> Self{
        Self{
            entries: Vec::with_capacity(1024), // Preallocate some space
            next_free: usize::MAX, // No free entries initially
        }
    
    }

    /// Allocate a new entry, resusing freed slots if available
    pub fn alloc(&mut self, task:T, deadline: u64) -> usize {

        //Case 1: There is a free slot in the middle
        if self.next_free != usize::MAX{
            let idx = self.next_free;

            // Read the next pointer from the free slot
            // and update the next_free pointer
            match self.entries[idx]{
                Entry::Free(next_idx) =>{
                    self.next_free = next_idx;
                }
                _ => panic!("Corrupted free list"),

            }
        // Write the data
        self.entries[idx] = Entry::Occupied(TimerEntry{
            task,
            deadline,
            next: None,
            prev: None,
        });
        return idx;
    }
    // Case 2: No Free slots, grow the vector
    let idx = self.entries.len();
    self.entries.push(Entry::Occupied(TimerEntry{
        task,
        deadline,
        next: None,
        prev: None,
    }));
    idx

    }

    pub fn free(&mut self, idx: usize) -> Option<T> {
        if idx >= self.entries.len() {
            return None;
        }

        // 1. Swap the data out (move it to return it)
        // 2. Replace it with Entry::Free(old_head)
        // 3. Update head to point to this index
        let new_state = Entry::Free(self.next_free);
        let old_state = std::mem::replace(&mut self.entries[idx], new_state);
        
        match old_state {
            Entry::Occupied(entry) => {
                self.next_free = idx; // This slot is now the head of free list
                Some(entry.task)
            }
            Entry::Free(_) => {
                // It was already free! Restore the state or panic.
                
                self.entries[idx] = old_state; 
                None 
            }
        }
    }

    pub fn get(&self, idx: usize) -> Option<&TimerEntry<T>> {
        match self.entries.get(idx) {
            Some(Entry::Occupied(entry)) => Some(entry),
            _ => None,
        }
    }
    
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut TimerEntry<T>> {
        match self.entries.get_mut(idx) {
            Some(Entry::Occupied(entry)) => Some(entry),
            _ => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut slab = Slab::new();
        
        let id_a = slab.alloc("Task A", 100);
        let id_b = slab.alloc("Task B", 200);

        assert_eq!(id_a, 0); // First item should be index 0
        assert_eq!(id_b, 1); // Second item should be index 1

        // Verify data integrity
        let entry_a = slab.get(id_a).unwrap();
        assert_eq!(entry_a.task, "Task A");
        assert_eq!(entry_a.deadline, 100);
    }

    #[test]
    fn test_reuse_slots() {
        // This is the CRITICAL test 
        let mut slab = Slab::new();
        
        let id_1 = slab.alloc(1, 10); // Index 0
        let id_2 = slab.alloc(2, 10); // Index 1
        let id_3 = slab.alloc(3, 10); // Index 2

        // Free the middle one (Index 1)
        let freed_val = slab.free(id_2);
        assert_eq!(freed_val, Some(2));

        // Now allocate a new one. It MUST reuse Index 1.
        let id_4 = slab.alloc(4, 10);
        
        assert_eq!(id_4, 1, "Slab did not reuse the freed slot!");
        
        // Allocate another. Should be Index 3 (since 1 is taken and 0,2 were never freed)
        let id_5 = slab.alloc(5, 10);
        assert_eq!(id_5, 3);
    }

    #[test]
    fn test_double_free_protection() {
        let mut slab = Slab::new();
        let id = slab.alloc("A", 10);

        // Free once
        assert!(slab.free(id).is_some());

        // Free again
        assert!(slab.free(id).is_none());
    }

    #[test]
    fn test_lifecycle() {
        let mut slab = Slab::new();
        
        // 1. Fill it up by forcing vector growth
        for i in 0..100 {
            slab.alloc(i, i as u64);
        }
        
        // 2. Free all even numbers
        for i in (0..100).step_by(2) {
            slab.free(i);
        }

        // 3. Allocate 50 new items. They should all fit in the even slots.
        // If the vector grows, this test logic implies the reuse failed.
        let capacity_before = slab.entries.capacity();
        
        for i in 0..50 {
            slab.alloc(i * 100, 0);
        }

               
        // Check that a known reused slot has new data
        let entry = slab.get(0).unwrap();
        assert_eq!(entry.task, 4900); // The new 0
    }
}