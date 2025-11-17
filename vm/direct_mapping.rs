//! Direct Address mapping.

use crate::virtual_memory::{LogicalAddress, PhysicalAddress};

use super::address_mapping::AddressMapping;

impl AddressMapping for DirectMapping {
    /// Obtains physical address for direct mapping.
    ///
    /// addr must be page aligned.
    fn allocAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress> {
        Some(addr.0)
    }

    /// Frees a physical address from a direct mapping.
    fn freeAddressMapping(addr: PhysicalAddress) {}

    /// Reserves space for a mapping.
    fn reserveAddressMapping(count: u32) -> Result<(), ()> {
        Ok(())
    }

    /// Frees reserved space.
    fn unreserveAddressMapping(count: u32) {}

    /// Allocates previously reserved space for a mapping.
    fn fulfillAddressMapping(addr: LogicalAddress) -> Option<PhysicalAddress> {
        Some(addr.0)
    }
}

/// Strategy for direct mapping.
pub struct DirectMapping;
