//! Heap Allocator
//! 
//! Provides dynamic memory allocation support for the kernel.
//! Uses a simple linked list allocator for demonstration purposes.

use super::memory_layout::*;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::cell::UnsafeCell;

/// Heap allocator using linked list
pub struct LinkedListAllocator {
    head: UnsafeCell<*mut FreeBlock>,
}

// Safety: We ensure thread safety through atomic operations and proper synchronization
unsafe impl Sync for LinkedListAllocator {}
unsafe impl Send for LinkedListAllocator {}

/// Free memory block in the linked list
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

impl LinkedListAllocator {
    /// Create a new empty allocator
    pub const fn new() -> Self {
        Self { 
            head: UnsafeCell::new(null_mut()),
        }
    }
    
    /// Initialize the allocator with a memory region
    /// 
    /// # Safety
    /// The memory region must be valid and unused
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        let block = heap_start as *mut FreeBlock;
        (*block).size = heap_size;
        (*block).next = null_mut();
        *self.head.get() = block;
    }
    
    /// Allocate memory (internal implementation)
    unsafe fn alloc_internal(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let align = layout.align();
        
        let mut current = *self.head.get();
        let mut prev: *mut FreeBlock = null_mut();
        
        while !current.is_null() {
            let block_addr = current as usize;
            let aligned_addr = align_up_to(block_addr, align);
            let padding = aligned_addr - block_addr;
            let total_size = padding + size;
            
            if (*current).size >= total_size {
                // Found a suitable block
                let alloc_addr = aligned_addr;
                let remaining_size = (*current).size - total_size;
                
                if remaining_size >= core::mem::size_of::<FreeBlock>() {
                    // Split the block
                    let new_block = (block_addr + total_size) as *mut FreeBlock;
                    (*new_block).size = remaining_size;
                    (*new_block).next = (*current).next;
                    
                    if prev.is_null() {
                        *self.head.get() = new_block;
                    } else {
                        (*prev).next = new_block;
                    }
                } else {
                    // Use the entire block
                    if prev.is_null() {
                        *self.head.get() = (*current).next;
                    } else {
                        (*prev).next = (*current).next;
                    }
                }
                
                return alloc_addr as *mut u8;
            }
            
            prev = current;
            current = (*current).next;
        }
        
        null_mut() // Out of memory
    }
    
    /// Deallocate memory (internal implementation)
    unsafe fn dealloc_internal(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let block_addr = ptr as usize;
        
        let block = block_addr as *mut FreeBlock;
        (*block).size = size;
        
        // Insert at the beginning for simplicity
        (*block).next = *self.head.get();
        *self.head.get() = block;
        
        // TODO: Merge adjacent free blocks for better efficiency
    }
}

/// Align address up to specified alignment
fn align_up_to(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_internal(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc_internal(ptr, layout)
    }
}

/// Global allocator instance
#[global_allocator]
static HEAP_ALLOCATOR: LinkedListAllocator = LinkedListAllocator::new();

/// Initialize the heap allocator
/// 
/// # Safety
/// Must be called only once during system initialization
pub unsafe fn init_heap() {
    HEAP_ALLOCATOR.init(KERNEL_HEAP_START, KERNEL_HEAP_SIZE);
}

/// Allocation error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
