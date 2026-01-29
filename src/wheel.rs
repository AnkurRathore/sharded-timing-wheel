use crate::slab::Slab;
use std::num::NonZeroU32;
// Constants: Use usize for shifting to avoid constant casting
const WHEEL_SIZE: usize = 64; // 2^6 slots per wheel
const WHEEL_BITS: usize = 6;
const NUM_LEVELS: usize = 4; // Supports up to 64^4 ticks
const WHEEL_MASK: u64 = 63; // 111111 binary

pub struct TimingWheel<T> {
    current_tick: u64,
    // 4 levels, 64 slots. Each slot holds the head Index of Linked List in the slab
    wheels: [[Option<NonZeroU32>; WHEEL_SIZE]; NUM_LEVELS],
    slab: Slab<T>,
}

impl<T> TimingWheel<T> {
    pub fn new() -> Self {
        Self {
            current_tick: 0,
            wheels: [[None; WHEEL_SIZE]; NUM_LEVELS],
            slab: Slab::new(),
        }
    }

    pub fn insert(&mut self, task: T, deadline: u64) -> NonZeroU32 {
        let duration = deadline.saturating_sub(self.current_tick);

        // 1. Determine which Level (Wheel) this belongs to
        let level = if duration < (1 << WHEEL_BITS) {
            0
        } else if duration < (1 << (2 * WHEEL_BITS)) {
            1
        } else if duration < (1 << (3 * WHEEL_BITS)) {
            2
        } else {
            3
        };

        // 2. Determine Which Slot (Bucket)
        let shift = level * WHEEL_BITS;
        let slot = ((deadline >> shift) & WHEEL_MASK) as usize;

        // 3. Allocate in the slab
        let new_idx = self.slab.alloc(task, deadline, level as u8);

        // 4. Intrusive Linked List Insertion at the head of the slot
        let old_head_idx = self.wheels[level][slot];

        // Update the NEW entry's pointers
        if let Some(mut entry) = self.slab.get_mut(new_idx) {
            entry.next = old_head_idx;
            entry.prev = None;
        }

        // Update the OLD head's prev pointer
        if let Some(old_idx) = old_head_idx {
            if let Some(mut old_head) = self.slab.get_mut(old_idx) {
                old_head.prev = Some(new_idx);
            }
        }

        // Update the wheel bucket to point to the new entry
        self.wheels[level][slot] = Some(new_idx);

        new_idx
    }

    pub fn cancel(&mut self, idx: NonZeroU32) -> Option<T> {
        // 1. Read metadata to find where this entry lives
        let (prev, next, deadline, level) = {
            let entry = self.slab.get(idx)?;
            (entry.prev, entry.next, entry.deadline, entry.level)
        };

        // re-calculate slot again just to update the wheel head if needed
        let shift = (level as usize) * WHEEL_BITS;
        let slot = ((deadline >> shift) & WHEEL_MASK) as usize;

        // 2. Unlink from "Prev"
        if let Some(prev_idx) = prev {
            if let Some(mut prev_entry) = self.slab.get_mut(prev_idx) {
                prev_entry.next = next;
            }
        } else {
            self.wheels[level as usize][slot] = next;
        }

        // 3. Unlink from "Next"
        if let Some(next_idx) = next {
            if let Some(mut next_entry) = self.slab.get_mut(next_idx) {
                next_entry.prev = prev;
            }
        }

        // 4. Finally free the memory and return task
        self.slab.free(idx)
    }

    pub fn process_bucket(&mut self, level: usize, slot: usize, expired: &mut Vec<T>) {
        // STEAL the list. The bucket is now empty (None).
        // This allows us to modify the slab while iterating the stolen indices.
        let mut next_idx = self.wheels[level][slot].take();

        // Walk the linked list
        while let Some(curr_idx) = next_idx {
            // 1. Get metadata and drop reference
            let (deadline, next_node) = {
                let entry = self.slab.get(curr_idx).unwrap();
                (entry.deadline, entry.next)
            };

            // 2. Logic: Expire or Cascade
            if deadline <= self.current_tick {
                // Expired: Remove and return
                if let Some(task) = self.slab.free(curr_idx) {
                    expired.push(task);
                }
            } else {
                // Not expired! Re-insert to the correct wheel (Cascading).
                // extract the task and re-insert it. This handles the new level calculation.
                if let Some(task) = self.slab.free(curr_idx) {
                    self.insert(task, deadline);
                }
            }

            // 3. Move to next
            next_idx = next_node;
        }
    }

    /// Core Tick Algorithm
    /// Advances time by 1 tick and returns all expired timers
    pub fn tick(&mut self, expired: &mut Vec<T>) {
        // Step 1: Process Level 0, current slot
        let slot0 = (self.current_tick & WHEEL_MASK) as usize;
        self.process_bucket(0, slot0, expired);
        // Step 2: Advance current tick
        self.current_tick += 1;

        // Step 3: Cascade Check
        let tick = self.current_tick;

        // Check level 1 (Wrapped if lower 6 bits are 0)
        if (tick & WHEEL_MASK) == 0 {
            let slot1 = ((tick >> WHEEL_BITS) & WHEEL_MASK) as usize;
            self.process_bucket(1, slot1, expired);
        }

        // Check level 2 (Wrapped if lower 12 bits are 0)
        // Use 1u64 to ensure type safety during shift
        if (tick & ((1u64 << (2 * WHEEL_BITS)) - 1)) == 0 {
            let slot2 = ((tick >> (2 * WHEEL_BITS)) & WHEEL_MASK) as usize;
            self.process_bucket(2, slot2, expired);
        }

        // Check level 3 (Wrapped if lower 18 bits are 0)
        if (tick & ((1u64 << (3 * WHEEL_BITS)) - 1)) == 0 {
            let slot3 = ((tick >> (3 * WHEEL_BITS)) & WHEEL_MASK) as usize;
            self.process_bucket(3, slot3, expired);
        }
    }

    pub fn current_time(&self) -> u64 {
        self.current_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert_and_tick() {
        let mut wheel = TimingWheel::new();
        let mut expired = Vec::new(); // 1. Create the buffer

        wheel.insert("task1", 5);
        wheel.insert("task2", 10);
        wheel.insert("task3", 2);

        // Tick 0 -> 1
        wheel.tick(&mut expired);
        wheel.tick(&mut expired);
        assert!(expired.is_empty()); // Verify nothing happened yet

        // Tick 2 (Should fire task3)
        wheel.tick(&mut expired);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "task3");

        // Clear the buffer to check the next specific event
        expired.clear();

        // Tick 3 -> 4
        wheel.tick(&mut expired);
        wheel.tick(&mut expired);

        // Tick 5 (Should fire task1)
        wheel.tick(&mut expired);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "task1");
    }

    #[test]
    fn test_cascade_from_wheel_1() {
        let mut wheel = TimingWheel::new();
        let mut expired = Vec::new();

        // Insert timer beyond first wheel (> 64 ticks)
        wheel.insert("far_future", 100);

        // Tick 99 times
        for _ in 0..100 {
            wheel.tick(&mut expired); // Pass the buffer
        }

        // Ensure nothing fired early
        assert!(expired.is_empty());

        // At tick 100, it should expire
        wheel.tick(&mut expired);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "far_future");
    }

    #[test]
    fn test_cancel() {
        let mut wheel = TimingWheel::new();
        let mut expired = Vec::new();

        let id1 = wheel.insert("task1", 5);
        let _id2 = wheel.insert("task2", 10);

        let cancelled = wheel.cancel(id1);
        assert_eq!(cancelled, Some("task1"));

        for _ in 0..=10 {
            wheel.tick(&mut expired);
            // accumulate all expired tasks in the loop.
            // "task1" should NEVER appear in this list.
            assert!(!expired.contains(&"task1"));
        }

        // "task2" should be there though (at tick 10)
        assert!(expired.contains(&"task2"));
    }
}
