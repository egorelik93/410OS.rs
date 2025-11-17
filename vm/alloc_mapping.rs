//! Address mapping based on allocating new frames.

use crate::virtual_memory::{LogicalAddress, PhysicalAddress};

use super::address_mapping::AddressMapping;
use super::frame_alloc::*;

impl AddressMapping for AllocMapping {
    /// Map a logical address to a physical address
    /// by allocating a new frame.
    fn allocAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress> {
        allocFrame()
    }

    /// Frees frame allocated for an address mapping.
    fn freeAddressMapping(addr: PhysicalAddress) {
        freeFrame(addr);
    }

    /// Reserves space for an address mapping.
    fn reserveAddressMapping(count: u32) -> Result<(), ()> {
        reserveFrames(count)
    }

    fn unreserveAddressMapping(count: u32) {
        unreserveFrames(count);
    }

    fn fulfillAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress> {
        fulfillReservedFrame()
    }
}

/// Access the alloc-based mapping strategy.
pub struct AllocMapping;
