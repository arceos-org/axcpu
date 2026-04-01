//! Trap handling.

pub use linkme::{
    distributed_slice as def_trap_handler, distributed_slice as register_trap_handler,
};
use memory_addr::VirtAddr;
pub use page_table_entry::MappingFlags as PageFaultFlags;

pub use crate::TrapFrame;

/// A slice of IRQ handler functions.
#[def_trap_handler]
pub static IRQ: [fn(usize) -> bool];

/// A slice of page fault handler functions.
#[def_trap_handler]
pub static PAGE_FAULT: [fn(VirtAddr, PageFaultFlags) -> bool];

/// A slice of breakpoint handler functions.
///
/// Each handler is invoked with a mutable reference to the trapped [`TrapFrame`]
/// and must return a boolean indicating whether it has fully handled the trap:
///
/// - `true` means the breakpoint has been handled and control should resume
///   according to the state encoded in the trap frame.
/// - `false` means the breakpoint was not handled and default processing
///   (such as falling back to another mechanism or terminating) should occur.
///
/// When returning `true`, the handler is responsible for updating the saved
/// program counter (or equivalent PC field) in the trap frame as required by
/// the target architecture. In particular, the handler must ensure that,
/// upon resuming from the trap, execution does not immediately re-trigger the
/// same breakpoint instruction or condition, which could otherwise lead to an
/// infinite trap loop. The exact way to advance or modify the PC is
/// architecture-specific and depends on how [`TrapFrame`] encodes the saved
/// context.
#[def_trap_handler]
pub static BREAK_HANDLER: [fn(&mut TrapFrame) -> bool];

/// A slice of debug handler functions.
///
/// On `x86_64`, these handlers are invoked for debug-related traps (for
/// example, hardware breakpoints, single-step traps, or other debug
/// exceptions). The handler receives a mutable reference to the trapped
/// [`TrapFrame`] and returns a boolean with the following meaning:
///
/// - `true` means the debug trap has been fully handled and execution should
///   resume from the state stored in the trap frame.
/// - `false` means the debug trap was not handled and default/secondary
///   processing should take place.
///
/// As with [`BREAK_HANDLER`], when returning `true`, the handler must adjust
/// the saved program counter (or equivalent) in the trap frame if required by
/// the architecture so that resuming execution does not immediately cause the
/// same debug condition to fire again. Callers must take the architecture-
/// specific PC semantics into account when deciding how to advance or modify
/// the PC.
#[cfg(target_arch = "x86_64")]
#[def_trap_handler]
pub static DEBUG_HANDLER: [fn(&mut TrapFrame) -> bool];

#[allow(unused_macros)]
macro_rules! handle_trap {
    ($trap:ident, $($args:tt)*) => {{
        let mut iter = $crate::trap::$trap.iter();
        if let Some(func) = iter.next() {
            if iter.next().is_some() {
                warn!("Multiple handlers for trap {} are not currently supported", stringify!($trap));
            }
            func($($args)*)
        } else {
            warn!("No registered handler for trap {}", stringify!($trap));
            false
        }
    }}
}
