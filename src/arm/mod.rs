//! ARM32 (ARMv7-A) architecture-specific code.

#[macro_use]
mod macros;

mod context;

pub mod asm;
pub mod init;

#[cfg(target_os = "none")]
mod trap;

pub use self::context::{FpState, TaskContext, TrapFrame};
pub use self::init::{cpsr, current_mode, is_privileged, mode};
