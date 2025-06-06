//! Structures and functions for user space.

use memory_addr::VirtAddr;

use crate::TrapFrame;

/// Context to enter user space.
pub struct UspaceContext(TrapFrame);

impl UspaceContext {
    /// Creates an empty context with all registers set to zero.
    pub const fn empty() -> Self {
        unsafe { core::mem::MaybeUninit::zeroed().assume_init() }
    }

    /// Creates a new context with the given entry point, user stack pointer,
    /// and the argument.
    pub fn new(entry: usize, ustack_top: VirtAddr, arg0: usize) -> Self {
        use aarch64_cpu::registers::SPSR_EL1;
        let mut regs = [0; 31];
        regs[0] = arg0 as _;
        Self(TrapFrame {
            r: regs,
            usp: ustack_top.as_usize() as _,
            elr: entry as _,
            spsr: (SPSR_EL1::M::EL0t
                + SPSR_EL1::D::Masked
                + SPSR_EL1::A::Masked
                + SPSR_EL1::I::Unmasked
                + SPSR_EL1::F::Masked)
                .value,
        })
    }

    /// Creates a new context from the given [`TrapFrame`].
    pub const fn from(trap_frame: &TrapFrame) -> Self {
        Self(*trap_frame)
    }

    /// Gets the instruction pointer.
    pub const fn get_ip(&self) -> usize {
        self.0.elr as _
    }

    /// Gets the stack pointer.
    pub const fn get_sp(&self) -> usize {
        self.0.usp as _
    }

    /// Sets the instruction pointer.
    pub const fn set_ip(&mut self, pc: usize) {
        self.0.elr = pc as _;
    }

    /// Sets the stack pointer.
    pub const fn set_sp(&mut self, sp: usize) {
        self.0.usp = sp as _;
    }

    /// Sets the return value register.
    pub const fn set_retval(&mut self, r0: usize) {
        self.0.r[0] = r0 as _;
    }

    /// Enters user space.
    ///
    /// It restores the user registers and jumps to the user entry point
    /// (saved in `elr`).
    /// When an exception or syscall occurs, the kernel stack pointer is
    /// switched to `kstack_top`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it changes processor mode and the stack.
    pub unsafe fn enter_uspace(&self, kstack_top: VirtAddr) -> ! {
        crate::asm::disable_irqs();
        // We do not handle traps that occur at the current exception level,
        // so the kstack ptr(`sp_el1`) will not change during running in user space.
        // Then we don't need to save the `sp_el1` to the taskctx.
        unsafe {
            core::arch::asm!(
                "
                mov     sp, x1
                ldp     x30, x9, [x0, 30 * 8]
                ldp     x10, x11, [x0, 32 * 8]
                msr     sp_el0, x9
                msr     elr_el1, x10
                msr     spsr_el1, x11

                ldp     x28, x29, [x0, 28 * 8]
                ldp     x26, x27, [x0, 26 * 8]
                ldp     x24, x25, [x0, 24 * 8]
                ldp     x22, x23, [x0, 22 * 8]
                ldp     x20, x21, [x0, 20 * 8]
                ldp     x18, x19, [x0, 18 * 8]
                ldp     x16, x17, [x0, 16 * 8]
                ldp     x14, x15, [x0, 14 * 8]
                ldp     x12, x13, [x0, 12 * 8]
                ldp     x10, x11, [x0, 10 * 8]
                ldp     x8, x9, [x0, 8 * 8]
                ldp     x6, x7, [x0, 6 * 8]
                ldp     x4, x5, [x0, 4 * 8]
                ldp     x2, x3, [x0, 2 * 8]
                ldp     x0, x1, [x0]
                eret",
                in("x0") &self.0,
                in("x1") kstack_top.as_usize() ,
                options(noreturn),
            )
        }
    }
}
