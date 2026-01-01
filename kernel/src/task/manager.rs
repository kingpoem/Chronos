//! Task Manager
//! 
//! Manages all tasks in the system

use super::task::TaskControlBlock;
use super::TaskStatus;
use crate::config::MAX_APP_NUM;
use alloc::vec::Vec;

pub struct TaskManager {
    tasks: Vec<Option<TaskControlBlock>>,
    current_task: Option<usize>,
}

impl TaskManager {
    pub fn new() -> Self {
        // Use Vec instead of fixed array to avoid stack overflow
        // Vec will allocate on heap, which is safer for large structures
        let mut tasks = Vec::with_capacity(MAX_APP_NUM);
        for _ in 0..MAX_APP_NUM {
            tasks.push(None);
        }
        Self {
            tasks,
            current_task: None,
        }
    }
    
    pub fn add_task(&mut self, task: TaskControlBlock) -> usize {
        for (idx, slot) in self.tasks.iter_mut().enumerate() {
            if slot.as_ref().is_none() {
                *slot = Some(task);
                return idx;
            }
        }
        panic!("No available task slot");
    }
    
    pub fn remove_task(&mut self, pid: usize) {
        if let Some(slot) = self.tasks.get_mut(pid) {
            *slot = None;
        }
        if self.current_task == Some(pid) {
            self.current_task = None;
        }
    }
    
    pub fn get_task(&self, pid: usize) -> Option<&TaskControlBlock> {
        self.tasks.get(pid)?.as_ref()
    }
    
    pub fn get_task_mut(&mut self, pid: usize) -> Option<&mut TaskControlBlock> {
        self.tasks.get_mut(pid)?.as_mut()
    }
    
    pub fn get_current_task(&self) -> Option<usize> {
        self.current_task
    }
    
    pub fn set_current_task(&mut self, pid: Option<usize>) {
        self.current_task = pid;
    }
    
    pub fn find_next_task(&self, start: usize) -> Option<usize> {
        let len = self.tasks.len();
        for i in 0..len {
            let idx = (start + i + 1) % len;
            if let Some(Some(task)) = self.tasks.get(idx) {
                if task.task_status == TaskStatus::Ready {
                    return Some(idx);
                }
            }
        }
        None
    }
    
    pub fn mark_zombie(&mut self, pid: usize) {
        if let Some(Some(task)) = self.tasks.get_mut(pid) {
            task.task_status = TaskStatus::Zombie;
        }
    }
    
    pub fn task_count(&self) -> usize {
        self.tasks.iter().filter(|t: &&Option<TaskControlBlock>| t.is_some()).count()
    }
}

