//! Helper functions to initialize the CPU states on systems bootstrapping.

use memory_addr::PhysAddr;

/// ARM32 processor modes.
#[allow(dead_code)]
pub mod mode {
    /// User mode (USR) - 0x10
    pub const USR: u32 = 0x10;
    /// FIQ mode - 0x11
    pub const FIQ: u32 = 0x11;
    /// IRQ mode - 0x12
    pub const IRQ: u32 = 0x12;
    /// Supervisor mode (SVC) - 0x13
    pub const SVC: u32 = 0x13;
    /// Monitor mode (MON) - 0x16 (Security Extensions)
    pub const MON: u32 = 0x16;
    /// Abort mode (ABT) - 0x17
    pub const ABT: u32 = 0x17;
    /// Hypervisor mode (HYP) - 0x1A (Virtualization Extensions)
    pub const HYP: u32 = 0x1A;
    /// Undefined mode (UND) - 0x1B
    pub const UND: u32 = 0x1B;
    /// System mode (SYS) - 0x1F
    pub const SYS: u32 = 0x1F;
}

/// CPSR flag bits.
#[allow(dead_code)]
pub mod cpsr {
    /// IRQ disable bit (I)
    pub const IRQ_DISABLE: u32 = 1 << 7;
    /// FIQ disable bit (F)
    pub const FIQ_DISABLE: u32 = 1 << 6;
    /// Thumb state bit (T)
    pub const THUMB: u32 = 1 << 5;
    /// Mode bits mask
    pub const MODE_MASK: u32 = 0x1F;
}

/// Configures and enables the MMU on the current CPU.
///
/// It first sets `TTBR0`, `TTBR1`, `TTBCR` registers to the conventional values,
/// and then enables the MMU and caches by setting `SCTLR`.
///
/// # Safety
///
/// This function is unsafe as it changes the address translation configuration.
pub unsafe fn init_mmu(root_paddr: PhysAddr) {
    use aarch32_cpu::asm::{dsb, isb};
    use aarch32_cpu::register::{Sctlr, TlbIAll};
    use core::arch::asm;

    let root = root_paddr.as_usize() as u32;

    unsafe {
        // Set TTBR0 and TTBR1 to the same page table
        // Note: VMSA-specific registers (TTBR0/1, TTBCR, DACR) are not abstracted in aarch32-cpu
        // as it focuses on PMSA, so we use direct assembly here
        asm!("mcr p15, 0, {}, c2, c0, 0", in(reg) root); // TTBR0
        asm!("mcr p15, 0, {}, c2, c0, 1", in(reg) root); // TTBR1

        // Set TTBCR to use TTBR0 for all addresses (N=1)
        asm!("mcr p15, 0, {}, c2, c0, 2", in(reg) 0x1u32);

        // Set Domain Access Control Register (all domains to client mode)
        // Domain 0-15: 01 = Client (check page table permissions)
        asm!("mcr p15, 0, {}, c3, c0, 0", in(reg) 0x55555555u32);

        // Invalidate entire TLB using aarch32_cpu abstraction
        TlbIAll::write();

        // Synchronization barriers using aarch32_cpu abstractions
        // These include compiler fences for proper ordering
        dsb();
        isb();

        // Enable MMU, data cache, and instruction cache using type-safe SCTLR abstraction
        Sctlr::modify(|r| {
            r.set_m(true); // M bit: Enable MMU
            r.set_c(true); // C bit: Enable data cache
            r.set_i(true); // I bit: Enable instruction cache
        });

        // Final synchronization barriers to ensure MMU is fully enabled
        // and instruction pipeline is flushed
        dsb();
        isb();
    }
}

/// Initializes trap handling on the current CPU.
///
/// This function performs the following initialization steps:
/// 1. Sets the exception vector base address (VBAR) to our exception vector table
/// 2. Sets `TTBR0` to 0 to block low address access (user space disabled initially)
/// 3. Ensures proper CPU mode for exception handling
///
/// After calling this function, the CPU is ready to handle:
/// - IRQ interrupts
/// - Data aborts
/// - Prefetch aborts
/// - Undefined instruction exceptions
/// - Software interrupts (SVC)
pub fn init_trap() {
    unsafe extern "C" {
        fn exception_vector_base();
    }
    unsafe {
        // Set VBAR to point to our exception vector table
        crate::asm::write_exception_vector_base(exception_vector_base as *const () as usize);
        // Disable user space page table initially
        crate::asm::write_user_page_table(0.into());
    }
}

/// Reads the current exception level / CPU mode.
///
/// Returns the mode bits from CPSR.
#[inline]
pub fn current_mode() -> u32 {
    let cpsr: u32;
    unsafe { core::arch::asm!("mrs {}, cpsr", out(reg) cpsr) };
    cpsr & cpsr::MODE_MASK
}

/// Checks if the current CPU is running in privileged mode.
#[inline]
pub fn is_privileged() -> bool {
    let mode = current_mode();
    mode != mode::USR
}
