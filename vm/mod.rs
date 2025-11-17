pub(super) mod page;
pub(super) mod kernel_memory;
pub(super) mod directory;
pub(super) mod paging;
pub(super) mod vm_internal;
pub(super) mod page_entry;
pub(super) mod address_mapping;
pub(super) mod direct_mapping;
pub(super) mod alloc_mapping;
pub(super) mod manager;
pub(super) mod mapped_memory;
pub(super) mod memory_alloc;
pub(super) mod validate_memory;
mod frame_alloc;
mod invalidate_page;

use super::*;
