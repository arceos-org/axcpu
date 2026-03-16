//! ARM32 (ARMv7-A) architecture-specific code.

mod context;

pub mod asm;
pub mod init;

#[cfg(target_os = "none")]
mod trap;

#[cfg(feature = "uspace")]
pub mod uspace {
    // TODO: This module is currently empty, but it will contain user-space related code in the future.
    use crate::TrapFrame;

    /// Context to enter user space.
    pub struct UspaceContext(TrapFrame);
}

pub use self::context::{FpState, TaskContext, TrapFrame};
