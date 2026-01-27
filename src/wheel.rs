use slab::{Slab, TimerEntry};

const WHEEL_SIZE: usize = 64; //2^6 slots per wheel
const WHEEL_BITS: u32 = 6;
const NUM_LEVELS: usize = 4; //Supports upto 64^4 = 16,777,216 ticks
const WHEEL_MASK: u64 = 63;

pub struct TimingWheel<T>{
    current_tick: u64,
    // 4 levels, 64 slots.Each slot holds the head Index of Linked List in the slab
    wheels: [[Option<usize>; WHEEL_SIZE]; NUM_LEVELS],
    slab: Slab<T>,
}

impl<T> TimingWheel<T>{
    pub fn insert(&mut self, task: T, deadline: u64) -> usize {
        let duration = deadline.saturating_sub(self.current_tick);

        // 1. Determine which Level (Wheel) this belongs to
        let level = if duration < 64 {0}
                    else if duration < 64 * 64 {1}
                    else if duration < 64 * 64 * 64 {2}
                    else {3};

        // 2. Determine Which Slot  (Bucket)
        // Shift the deadline down to the unit of that level
        // Masking is applied to wrap around
        let shift = level * WHEEL_BITS;
        let slot = ((deadline >> shift) & WHEEL_MASK) as usize;

        // 3 Allocate in the slab
        let new_idx  = self.slab.allocate(task, deadline);

        // Intrusive Linked List Insertion at the head of the slot
        if let Some(mut entry) = self.slab.get_mut(new_idx){
            entry.next = self.wheels[level][slot];

        }

        // update the head
        self.wheels[level][slot] = Sonme(new_idx);

        new_idx

    }

    pub fn cancel(&mut self, idx: usize) -> Option<T>{
        // Get the entry to find the slot/wheel
        let entry = self.slab.get(idx)?;
        let deadline = entry.deadline;
        let duration = deadline.saturating_sub(self.current_tick);

        let level = if duration < 64 {0}
                    else if duration < 64 * 64 {1}
                    else if duration < 64 * 64 * 64 {2}
                    else {3};
        
        let shift = level * WHEEL_BITS;
        let slot = ((deadline >> shift) & WHEEL_MASK) as usize;

        let prev_idx = entry.prev;
        let next_idx = entry.next;

        // update the prev and next entries
        if let Some(prev_idx) = prev_idx{
            self.slab.get_mut(prev_idx).unwrap().next = next_idx;
        } else{
            self.wheels[level][slot]= next_idx;
        }
        if let Some(next_idx) = next_idx{
            self.slab.get_mut(next_idx).unwrap().prev = prev_idx;
        }

        // Free the slab entry
        self.slab.free(idx);

    }

    pub fn process_bucket(&mut self, level: usize, slot: usize) -> Vec<T>{
        let mut expired = Vec::new();
        
        // Take the current head of the slot
        let mut current_idx = self.wheels[level][slot].take();

        // Walk the linked list
        while let Some(idx) = current_idx{
            entry = self.slab.get(idx).unwrap();
            deadline = entry.deadline;
            next_idx = entry.next;

            // check if expired
            if deadline <= self.current_tick{
                // Expired, remove from wheel and slab
                if let (Some(task)) = self.slab.free(idx){
                    expired.push(task);
                
                }
            } else{
                // Not expired! Re-insert to the correct wheel
                let duration = deadline.saturating_sub(self.current_tick);

                // Calculate new position
                let duration = deadline.saturating_sub(self.current_tick);
                let new_level = if duration < 64 {0}
                                else if duration < 64 * 64 {1}
                                else if duration < 64 * 64 * 64 {2}
                                else {3};
                let shift = new_level * WHEEL_BITS;
                let new_slot = ((deadline >> shift) & WHEEL_MASK) as usize;

                // Reset the entry's pointers
                if let Some(entry) = self.slab.get_mut(idx){
                    entry.next = self.wheels[new_level][new_slot];
                    entry.prev = None;

              }
                // Update old head if exists
                if let Some(old_head_idx) = self.wheels[new_level][new_slot] {
                    if let Some(old_head) = self.slab.get_mut(old_head_idx) {
                        old_head.prev = Some(idx);
                    }
 
                }

                // Update the Wheel to point to this entry
                self.wheels[new_level][new_slot] = Some(idx);


        }
        //Move to the next entry
        current_idx = next_idx;
    }
    expired
    
    }
        
}