//! Scheduler
//! 
//! Implements round-robin scheduling

use super::manager::TaskManager;

pub struct Scheduler {
    time_slice: usize,
    current_time_slice: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            time_slice: 10, // 10 ticks per time slice (100ms at 10ms per tick)
            current_time_slice: 0,
        }
    }
    
    pub fn schedule_next(&mut self, current: usize, task_manager: &TaskManager) -> Option<usize> {
        // Round-robin: find next ready task
        task_manager.find_next_task(current)
    }
    
    pub fn tick(&mut self) -> bool {
        self.current_time_slice += 1;
        if self.current_time_slice >= self.time_slice {
            self.current_time_slice = 0;
            true // Time slice expired, need to switch
        } else {
            false
        }
    }
    
    pub fn reset_time_slice(&mut self) {
        self.current_time_slice = 0;
    }
}

