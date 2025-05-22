//! Structures and functions for user space.

use memory_addr::VirtAddr;

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
        const SPIE: usize = 1 << 5; // enable interrupts
        const SUM: usize = 1 << 18; // enable user memory access in supervisor mode
        Self(TrapFrame {
            regs: GeneralRegisters {
                a0: arg0,
                sp: ustack_top.as_usize(),
                ..Default::default()
            },
            sepc: entry,
            sstatus: SPIE | SUM,
        })
    }

    /// Creates a new context from the given [`TrapFrame`].
    pub const fn from(trap_frame: &TrapFrame) -> Self {
        Self(*trap_frame)
    }

    /// Gets the instruction pointer.
    pub const fn get_ip(&self) -> usize {
        self.0.sepc
    }

    /// Gets the stack pointer.
    pub const fn get_sp(&self) -> usize {
        self.0.regs.sp
    }

    /// Sets the instruction pointer.
    pub const fn set_ip(&mut self, pc: usize) {
        self.0.sepc = pc;
    }

    /// Sets the stack pointer.
    pub const fn set_sp(&mut self, sp: usize) {
        self.0.regs.sp = sp;
    }

    /// Sets the return value register.
    pub const fn set_retval(&mut self, a0: usize) {
        self.0.regs.a0 = a0;
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

                STR     gp, {kernel_trap_addr}, 2
                LDR     gp, sp, 2

                STR     tp, {kernel_trap_addr}, 3
                LDR     tp, sp, 3

                LDR     t0, sp, 32
                csrw    sstatus, t0
                POP_GENERAL_REGS
                LDR     sp, sp, 1
                sret",
                tf = in(reg) &(self.0),
                kernel_trap_addr = in(reg) kernel_trap_addr,
                options(noreturn),
            )
        }
    }
}
