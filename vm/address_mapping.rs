//! Interface for mapping between logical and physical addresses.

use crate::virtual_memory::{LogicalAddress, PhysicalAddress};

/// A strategy for allocating and freeing
/// a target for the mapping.
pub trait AddressMapping {
    /// Returns a physical address that can be used
    /// to map the logical address to.
    ///
    /// addr must be page aligned.
    fn allocAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress>;

    /// Frees any resources allocated by the corresponding
    /// call to allocMapping.
    fn freeAddressMapping(addr: PhysicalAddress);

    /// Reserves space for a mapping without
    /// actually allocating.
    fn reserveAddressMapping(count: u32) -> Result<(), ()>;

    /// Frees a space reservation
    fn unreserveAddressMapping(count: u32);

    /// Allocate the space for a previously reserved mapping.
    fn fulfillAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress>;
}
