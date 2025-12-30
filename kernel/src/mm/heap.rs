//! Heap Allocator
//!
//! Uses buddy system allocator for efficient memory management

use crate::config::memory_layout::*;
use buddy_system_allocator::LockedHeap;

/// Global allocator instance using Buddy System
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

/// Initialize the heap allocator
///
/// # Safety
/// Must be called only once during system initialization
pub unsafe fn init_heap() {
    HEAP_ALLOCATOR
        .lock()
        .init(KERNEL_HEAP_START, KERNEL_HEAP_SIZE);
}

/// Allocation error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
