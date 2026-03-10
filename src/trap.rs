//! Trap handling.

use memory_addr::VirtAddr;

pub use crate::TrapFrame;
pub use page_table_entry::MappingFlags as PageFaultFlags;

/// IRQ handler.
#[eii]
pub fn irq_handler(irq: usize) -> bool;

/// Page fault handler.
#[eii]
pub fn page_fault_handler(addr: VirtAddr, flags: PageFaultFlags) -> bool;
