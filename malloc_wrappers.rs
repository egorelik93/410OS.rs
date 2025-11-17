//! Safe versions of malloc functions

use core::alloc::{GlobalAlloc, Layout};

use _410kern::malloc_internal::*;

use crate::sync::mutex::Mutex;

static memoryMutex: Mutex<()> = Mutex::new(());

/// Allocates memory on the heap
pub fn malloc(size: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _malloc(size) }
}

/// Allocates a block of memory. The address is a multiple of alignment.
pub fn memalign(alignment: usize, size: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _memalign(alignment, size) }
}

/// Allocates memory on the heap for an array, and zeroes it out
pub fn calloc(nelt: usize, eltsize: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _calloc(nelt, eltsize) }
}

/// Change the size of an existing chunk of heap memory
pub fn realloc(buf: *mut u8, new_size: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _realloc(buf, new_size) }
}

/// Free an allocated memory block
pub fn free(buf: *mut u8) {
    let guard = memoryMutex.lock();
    unsafe { _free(buf); }
}

/// Alternative version of malloc. User must keep track of size.
pub fn smalloc(size: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _smalloc(size) }
}

/// Alternative version of memalign. User must keep track of size.
pub fn smemalign(alignment: usize, size: usize) -> *mut u8 {
    let guard = memoryMutex.lock();
    unsafe { _smemalign(alignment, size) }
}

/// Free a block of memory allocated using smalloc()
pub fn sfree(buf: *mut u8, size: usize) {
    let guard = memoryMutex.lock();
    unsafe { _sfree(buf, size); }
}


/// Use the malloc as the global allocator.
///
/// This was not in the original C implementation, but
/// allows us to use Rust niceties like Box.
/// This is thanks to the relatively recent no_global_oom_handling,
/// which lets use a safe non-OOM version of the alloc crate.
/// This was not available when I first attempted a Rust port,
/// and is exactly where I stalled.
struct MallocAlloc;

unsafe impl GlobalAlloc for MallocAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        smemalign(layout.align(), layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        sfree(ptr, layout.size());
    }
}

#[global_allocator]
static malloc_alloc: MallocAlloc = MallocAlloc;
