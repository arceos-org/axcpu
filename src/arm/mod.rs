//! ARM32 (ARMv7-A) architecture-specific code.

mod context;

pub mod asm;
pub mod init;

#[cfg(target_os = "none")]
mod trap;

pub use self::asm::{current_mode, is_privileged};
pub use self::context::{FpState, TaskContext, TrapFrame};
pub use self::init::{cpsr, mode};
