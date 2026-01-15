//! Memory mapping system calls
//!
//! Implements mmap and munmap system calls for memory mapping

use crate::mm::memory_set::{MapArea, MapPermission, MapType};
use crate::config::memory_layout::PAGE_SIZE;
use crate::task::TASK_MANAGER;

/// Protection flags (from Linux)
pub const PROT_READ: usize = 0x1;
pub const PROT_WRITE: usize = 0x2;
pub const PROT_EXEC: usize = 0x4;

/// Mapping flags (from Linux)
#[allow(dead_code)]
pub const MAP_PRIVATE: usize = 0x02;
#[allow(dead_code)]
pub const MAP_SHARED: usize = 0x01;
pub const MAP_ANONYMOUS: usize = 0x20;
pub const MAP_FIXED: usize = 0x10;

/// Error return value (same as Linux MAP_FAILED)
pub const MAP_FAILED: isize = -1;

/// Map memory region
/// 
/// # Arguments
/// * `addr` - Suggested virtual address (0 means let kernel choose)
/// * `length` - Size of mapping in bytes
/// * `prot` - Protection flags (PROT_READ, PROT_WRITE, PROT_EXEC)
/// * `flags` - Mapping flags (MAP_PRIVATE, MAP_SHARED, MAP_ANONYMOUS, MAP_FIXED)
/// * `fd` - File descriptor (ignored for anonymous mappings)
/// * `offset` - File offset (ignored for anonymous mappings)
/// 
/// # Returns
/// * Success: Virtual address of mapped region
/// * Failure: MAP_FAILED (-1)
pub fn sys_mmap(
    addr: usize,
    length: usize,
    prot: usize,
    flags: usize,
    _fd: usize,
    _offset: usize,
) -> isize {
    // Only support anonymous mappings for now
    if (flags & MAP_ANONYMOUS) == 0 {
        crate::sbi::console_putstr("[mmap] Error: Only MAP_ANONYMOUS is supported\n");
        return MAP_FAILED;
    }
    
    // Validate length
    if length == 0 {
        crate::sbi::console_putstr("[mmap] Error: length must be > 0\n");
        return MAP_FAILED;
    }
    
    // Align length to page boundary
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    
    // Get current task
    let mut task_manager = TASK_MANAGER.lock();
    let current_pid = match task_manager.get_current_task() {
        Some(pid) => pid,
        None => {
            crate::sbi::console_putstr("[mmap] Error: No current task\n");
            return MAP_FAILED;
        }
    };
    
    let task = match task_manager.get_task_mut(current_pid) {
        Some(task) => task,
        None => {
            crate::sbi::console_putstr("[mmap] Error: Task not found\n");
            return MAP_FAILED;
        }
    };
    
    // Determine virtual address
    let virt_addr = if addr != 0 && (flags & MAP_FIXED) != 0 {
        // Use specified address (must be page-aligned)
        if addr % PAGE_SIZE != 0 {
            crate::sbi::console_putstr("[mmap] Error: addr must be page-aligned\n");
            return MAP_FAILED;
        }
        addr
    } else if addr != 0 {
        // Try to use suggested address (if available)
        addr & !(PAGE_SIZE - 1) // Align to page boundary
    } else {
        // Let kernel choose an address
        // For simplicity, we'll use a fixed region above user stack
        // In a real OS, this would be more sophisticated
        const MMAP_START: usize = 0x20000000; // Start of mmap region
        
        // Find a free region (simplified: just use MMAP_START for now)
        // TODO: Implement proper address space management
        MMAP_START
    };
    
    // Check if the region overlaps with existing mappings
    // For simplicity, we'll just check if the start address is valid
    // In a real OS, we'd check the entire range
    let start_va = virt_addr;
    let end_va = start_va + aligned_length;
    
    // Convert protection flags to MapPermission
    let mut perm = MapPermission::U; // User mode
    if (prot & PROT_READ) != 0 {
        perm |= MapPermission::R;
    }
    if (prot & PROT_WRITE) != 0 {
        perm |= MapPermission::W;
    }
    if (prot & PROT_EXEC) != 0 {
        perm |= MapPermission::X;
    }
    
    // Create map area
    let map_area = MapArea::new(start_va, end_va, MapType::Framed, perm);
    
    // Add to memory set
    task.memory_set.push(map_area, None);
    
    // Flush TLB if needed (sfence.vma is done in activate, but we're not switching address space)
    // For now, we'll rely on the next address space switch to flush TLB
    
    drop(task_manager);
    
    crate::sbi::console_putstr("[mmap] Mapped region: 0x");
    crate::trap::print_hex_usize(start_va);
    crate::sbi::console_putstr(" - 0x");
    crate::trap::print_hex_usize(end_va);
    crate::sbi::console_putstr(" (length: ");
    crate::trap::print_hex_usize(aligned_length);
    crate::sbi::console_putstr(")\n");
    
    start_va as isize
}

/// Unmap memory region
/// 
/// # Arguments
/// * `addr` - Virtual address of mapped region (must be page-aligned)
/// * `length` - Size of region to unmap in bytes
/// 
/// # Returns
/// * Success: 0
/// * Failure: -1
pub fn sys_munmap(addr: usize, length: usize) -> isize {
    // Validate address
    if addr % PAGE_SIZE != 0 {
        crate::sbi::console_putstr("[munmap] Error: addr must be page-aligned\n");
        return -1;
    }
    
    // Validate length
    if length == 0 {
        crate::sbi::console_putstr("[munmap] Error: length must be > 0\n");
        return -1;
    }
    
    // Align length to page boundary
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let start_va = addr;
    let end_va = start_va + aligned_length;
    
    // Get current task
    let mut task_manager = TASK_MANAGER.lock();
    let current_pid = match task_manager.get_current_task() {
        Some(pid) => pid,
        None => {
            crate::sbi::console_putstr("[munmap] Error: No current task\n");
            return -1;
        }
    };
    
    let task = match task_manager.get_task_mut(current_pid) {
        Some(task) => task,
        None => {
            crate::sbi::console_putstr("[munmap] Error: Task not found\n");
            return -1;
        }
    };
    
    // Find and remove the map area that contains this address
    // We need to find the area that contains [start_va, end_va)
    let mut found = false;
    let mut area_index = 0;
    
    for (i, area) in task.memory_set.areas().iter().enumerate() {
        let area_start = area.start_va();
        let area_end = area.end_va();
        
        // Check if the region to unmap overlaps with this area
        if start_va < area_end && end_va > area_start {
            // Found overlapping area
            // For simplicity, we'll only unmap if the region exactly matches an area
            // In a real OS, we'd handle partial unmapping
            if start_va == area_start && end_va == area_end {
                area_index = i;
                found = true;
                break;
            } else {
                // Partial unmapping not supported yet
                crate::sbi::console_putstr("[munmap] Error: Partial unmapping not supported\n");
                crate::sbi::console_putstr("[munmap] Requested: 0x");
                crate::trap::print_hex_usize(start_va);
                crate::sbi::console_putstr(" - 0x");
                crate::trap::print_hex_usize(end_va);
                crate::sbi::console_putstr(", Area: 0x");
                crate::trap::print_hex_usize(area_start);
                crate::sbi::console_putstr(" - 0x");
                crate::trap::print_hex_usize(area_end);
                crate::sbi::console_putstr("\n");
                return -1;
            }
        }
    }
    
    if !found {
        crate::sbi::console_putstr("[munmap] Error: Region not found\n");
        crate::sbi::console_putstr("[munmap] Requested: 0x");
        crate::trap::print_hex_usize(start_va);
        crate::sbi::console_putstr(" - 0x");
        crate::trap::print_hex_usize(end_va);
        crate::sbi::console_putstr("\n");
        return -1;
    }
    
    // Remove the area
    task.memory_set.remove_area(area_index);
    
    drop(task_manager);
    
    crate::sbi::console_putstr("[munmap] Unmapped region: 0x");
    crate::trap::print_hex_usize(start_va);
    crate::sbi::console_putstr(" - 0x");
    crate::trap::print_hex_usize(end_va);
    crate::sbi::console_putstr("\n");
    
    0
}

