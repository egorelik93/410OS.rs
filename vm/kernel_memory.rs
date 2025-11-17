//! Location of kernel page directory

use _410kern::page::PAGE_SIZE;

pub const KERNEL_DIRECTORY: usize = PAGE_SIZE;
