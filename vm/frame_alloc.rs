//! Allocates frames from physical memory.

use core::pin::Pin;

use _410kern::page::PAGE_SIZE;
use alloc::boxed::Box;

use crate::lprintf;
use crate::sync::mutex::Mutex;
use crate::virtual_memory::{
    LogicalAddress,
    PAGE_FREE,
    PageDirectory,
    PhysicalAddress,
    isPageAligned};

struct FrameAllocator(Mutex<FrameAllocatorInner>);

struct FrameAllocatorInner {
    // In the original, FrameAllocator did not own the kernelDirectory, but
    // given the usage it makes sense here.
    kernelDirectory: Option<Pin<Box<PageDirectory>>>,
    regionStart: PhysicalAddress,
    regionEnd: PhysicalAddress,
    currFrame: PhysicalAddress,
    bytesFree: usize
}

static allocator: FrameAllocator = FrameAllocator(Mutex::new(FrameAllocatorInner {
    kernelDirectory: None,
    regionStart: 0,
    regionEnd: 0,
    currFrame: 0,
    bytesFree: 0 }));


/// Set up frame allocation from the given region.
///
/// Frames will only be allocated from the
/// between start and end.
///
/// In the original, this function did not take over kernelDirectory,
/// but given the usage it works better with borrowing.
pub fn initFrameAllocator(kernelDirectory: Pin<Box<PageDirectory>>, start: PhysicalAddress, end: PhysicalAddress) {
    let mut guard = allocator.0.lock();
    guard.kernelDirectory = Some(kernelDirectory);
    guard.currFrame = start;
    guard.regionStart = start;
    guard.regionEnd = end;
    guard.bytesFree = end - start;

    // Mark regions between start and end as free
    let mut addr = start;
    while addr < end {
        let entry = unsafe { guard.kernelDirectory.tryGetPageEntryMut(LogicalAddress(start)).unwrap() };
        *entry = *entry | PAGE_FREE;
        addr = addr + PAGE_SIZE;
    }
}

/// Allocates a new physical frame.
pub fn allocFrame() -> Option<PhysicalAddress> {
    reserveFrames(1)?;
    fulfillReservedFrame()
}

/// Frees an allocated physical frame.
///
/// Has no effect if frame is
/// not in the allocation region.
pub fn freeFrame(frame: PhysicalAddress) {
    assert!(isPageAligned(LogicalAddress(frame)));

    let guard = allocator.0.lock();

    if guard.regionStart <= frame && frame < guard.regionEnd {
        let entry = guard.kernelDirectory.tryGetPageEntryMut(LogicalAddress(frame));

        *entry = *entry | PAGE_FREE;
        guard.bytesFree += PAGE_SIZE;
    } else {
        lprintf!("ILLEGAL: Trying to free a frame outside region.\n")
    }
}

/// Reserves some number of frames without
/// actually allocating.
pub fn reserveFrames(count: i32) -> Result<(), ()> {
    let mut guard = allocator.0.lock();

    if guard.bytesFree >= count * PAGE_SIZE {
        guard.bytesFree -= count * PAGE_SIZE;
        Some(())
    } else {
        return Err(())
    }
}

/// Frees up reserved frames.
pub fn unreserveFrames(count: i32) {
    let mut guard = allocator.0.lock();

    guard.bytesFree += count * PAGE_SIZE;
}

/// Allocates a new physical frame.
///
/// This should only be called after having already
/// reserved a frame. For getting a new frame
/// immediately, use allocFrame().
pub fn fulfillReservedFrame() -> Option<PhysicalAddress> {
    let guard = allocator.0.lock();

    let mut curr = guard.currFrame + PAGE_SIZE;

    while curr != guard.currFrame {
        let entry = unsafe { guard.kernelDirectory.tryGetPageEntryMut(LogicalAddress(curr)) };

        if let Some(entry) = entry && entry.page_is_free() {
            *entry = *entry & !PAGE_FREE;
            guard.currFrame = curr;

            return Some(curr);
        }

        curr = curr + PAGE_SIZE;
        if curr >= guard.regionEnd {
            curr = guard.regionStart;
        }
    }

    None
}
