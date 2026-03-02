//! Helper functions to initialize the CPU states on systems bootstrapping.

use crate::asm;
use memory_addr::PhysAddr;

/// Configures and enables the MMU on the current CPU.
///
/// It first sets `TTBR0`, `TTBR1`, `TTBCR` registers to the conventional values,
/// and then enables the MMU and caches by setting `SCTLR`.
///
/// # Safety
///
/// This function is unsafe as it changes the address translation configuration.
pub unsafe fn init_mmu(root_paddr: PhysAddr) {
    use aarch32_cpu::register::Sctlr;

    unsafe {
        // Set TTBR0 and TTBR1 to the same page table
        asm::write_user_page_table(root_paddr); // TTBR0
        asm::write_kernel_page_table(root_paddr); // TTBR1

        // Set TTBCR.N = 1: split VA space 2 GiB/2 GiB (low half via TTBR0, high half via TTBR1)
        asm::write_ttbcr(0x1u32);

        // Set Domain Access Control Register (all domains to client mode)
        // Domain 0-15: 01 = Client (check page table permissions)
        asm::write_dacr(0x55555555u32);

        // Invalidate entire TLB
        asm::flush_tlb(None);

        // Enable MMU, data cache, and instruction cache
        Sctlr::modify(|r| {
            r.set_m(true); // M bit: Enable MMU
            r.set_c(true); // C bit: Enable data cache
            r.set_i(true); // I bit: Enable instruction cache
        });

        // Final synchronization barriers to ensure MMU is fully enabled
        // and instruction pipeline is flushed
        asm::dsb();
        asm::isb();
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
