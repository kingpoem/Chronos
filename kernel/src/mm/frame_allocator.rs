//! Physical Frame Allocator
//!
//! Manages physical memory pages using a bitmap allocation strategy.

use crate::config::memory_layout::*;
use crate::mm::memory_layout::*;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Maximum number of physical frames (128MB / 4KB = 32K frames)
const MAX_FRAMES: usize = 32768;

/// Bitmap-based frame allocator
pub struct BitmapFrameAllocator {
    /// Start physical page number
    start_ppn: usize,
    /// End physical page number
    end_ppn: usize,
    /// Bitmap for tracking allocated frames (1 = allocated, 0 = free)
    bitmap: [AtomicUsize; MAX_FRAMES / (core::mem::size_of::<usize>() * 8)],
    /// Next frame to try allocating (optimization)
    next: AtomicUsize,
}

impl BitmapFrameAllocator {
    /// Create a new frame allocator
    pub const fn new() -> Self {
        const ATOMIC_ZERO: AtomicUsize = AtomicUsize::new(0);
        Self {
            start_ppn: 0,
            end_ppn: 0,
            bitmap: [ATOMIC_ZERO; MAX_FRAMES / (core::mem::size_of::<usize>() * 8)],
            next: AtomicUsize::new(0),
        }
    }

    /// Initialize the frame allocator with memory range
    ///
    /// # Safety
    /// Must be called only once during system initialization
    pub unsafe fn init(&mut self, start: usize, end: usize) {
        let start_ppn = align_up(start) >> PAGE_SIZE_BITS;
        let end_ppn = align_down(end) >> PAGE_SIZE_BITS;

        self.start_ppn = start_ppn;
        self.end_ppn = end_ppn;
        self.next.store(0, Ordering::Relaxed);

        // Clear all bitmap entries
        for entry in self.bitmap.iter() {
            entry.store(0, Ordering::Relaxed);
        }
    }

    /// Allocate a physical frame
    pub fn alloc(&self) -> Option<PhysPageNum> {
        let total_frames = self.end_ppn - self.start_ppn;
        let start_idx = self.next.load(Ordering::Relaxed);

        // Search from next position
        for offset in 0..total_frames {
            let idx = (start_idx + offset) % total_frames;
            let bitmap_idx = idx / (core::mem::size_of::<usize>() * 8);
            let bit_idx = idx % (core::mem::size_of::<usize>() * 8);

            let old_val = self.bitmap[bitmap_idx].load(Ordering::Acquire);
            let mask = 1usize << bit_idx;

            // If bit is 0 (free), try to set it to 1 (allocated)
            if (old_val & mask) == 0 {
                let new_val = old_val | mask;
                if self.bitmap[bitmap_idx]
                    .compare_exchange(old_val, new_val, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    // Successfully allocated
                    self.next.store((idx + 1) % total_frames, Ordering::Relaxed);
                    let ppn = self.start_ppn + idx;

                    // Zero out the frame for security
                    let ptr = (ppn << PAGE_SIZE_BITS) as *mut u8;
                    unsafe {
                        core::ptr::write_bytes(ptr, 0, PAGE_SIZE);
                    }

                    return Some(PhysPageNum::new(ppn));
                }
            }
        }

        None // Out of memory
    }

    /// Deallocate a physical frame
    pub fn dealloc(&self, ppn: PhysPageNum) {
        let ppn_val = ppn.as_usize();
        if ppn_val < self.start_ppn || ppn_val >= self.end_ppn {
            return; // Invalid frame
        }

        let idx = ppn_val - self.start_ppn;
        let bitmap_idx = idx / (core::mem::size_of::<usize>() * 8);
        let bit_idx = idx % (core::mem::size_of::<usize>() * 8);

        let mask = !(1usize << bit_idx);
        self.bitmap[bitmap_idx].fetch_and(mask, Ordering::Release);
    }

    /// Get number of free frames
    pub fn free_frames(&self) -> usize {
        let total_frames = self.end_ppn - self.start_ppn;
        let mut free_count = 0;

        for idx in 0..total_frames {
            let bitmap_idx = idx / (core::mem::size_of::<usize>() * 8);
            let bit_idx = idx % (core::mem::size_of::<usize>() * 8);
            let val = self.bitmap[bitmap_idx].load(Ordering::Relaxed);

            if (val & (1usize << bit_idx)) == 0 {
                free_count += 1;
            }
        }

        free_count
    }

    /// Get total number of frames
    pub fn total_frames(&self) -> usize {
        self.end_ppn - self.start_ppn
    }
}

/// Global frame allocator instance
pub static FRAME_ALLOCATOR: BitmapFrameAllocator = BitmapFrameAllocator::new();

/// Initialize the global frame allocator
///
/// # Safety
/// Must be called only once during system initialization
pub unsafe fn init(start: usize, end: usize) {
    let allocator = &FRAME_ALLOCATOR as *const _ as *mut BitmapFrameAllocator;
    (*allocator).init(start, end);
}

/// Frame allocator trait for abstraction
pub trait FrameAllocator {
    fn alloc(&self) -> Option<PhysPageNum>;
    fn dealloc(&self, ppn: PhysPageNum);
}

impl FrameAllocator for BitmapFrameAllocator {
    fn alloc(&self) -> Option<PhysPageNum> {
        self.alloc()
    }

    fn dealloc(&self, ppn: PhysPageNum) {
        self.dealloc(ppn)
    }
}
