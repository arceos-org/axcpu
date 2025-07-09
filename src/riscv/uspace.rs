//! Structures and functions for user space.

use memory_addr::VirtAddr;
use riscv::register::sstatus::Sstatus;
#[cfg(feature = "fp-simd")]
use riscv::register::sstatus::FS;

use crate::{GeneralRegisters, TrapFrame};

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
        let mut sstatus = Sstatus::from_bits(0);
        sstatus.set_spie(true); // enable interrupts
        sstatus.set_sum(true); // enable user memory access in supervisor mode
        #[cfg(feature = "fp-simd")]
        {
            sstatus.set_fs(FS::Initial); // set the FPU to initial state
        }

        Self(TrapFrame {
            regs: GeneralRegisters {
                a0: arg0,
                sp: ustack_top.as_usize(),
                ..Default::default()
            },
            sepc: entry,
            sstatus,
        })
    }

    /// Creates a new context from the given [`TrapFrame`].
    pub const fn from(trap_frame: &TrapFrame) -> Self {
        Self(*trap_frame)
    }

    /// Enters user space.
    ///
    /// It restores the user registers and jumps to the user entry point
    /// (saved in `sepc`).
    /// When an exception or syscall occurs, the kernel stack pointer is
    /// switched to `kstack_top`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it changes processor mode and the stack.
    pub unsafe fn enter_uspace(&self, kstack_top: VirtAddr) -> ! {
        use riscv::register::{sepc, sscratch};

        crate::asm::disable_irqs();
        // Address of the top of the kernel stack after saving the trap frame.
        let kernel_trap_addr = kstack_top.as_usize() - core::mem::size_of::<TrapFrame>();
        unsafe {
            sscratch::write(kstack_top.as_usize());
            sepc::write(self.0.sepc);
            core::arch::asm!(
                include_asm_macros!(),
                "
                mv      sp, {tf}

                STR     gp, {kernel_trap_addr}, 3
                LDR     gp, sp, 3

                STR     tp, {kernel_trap_addr}, 4
                LDR     tp, sp, 4

                LDR     t0, sp, 33
                csrw    sstatus, t0
                POP_GENERAL_REGS
                LDR     sp, sp, 2
                sret",
                tf = in(reg) &(self.0),
                kernel_trap_addr = in(reg) kernel_trap_addr,
                options(noreturn),
            )
        }
    }
}

impl core::ops::Deref for UspaceContext {
    type Target = TrapFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for UspaceContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
