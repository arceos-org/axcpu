//! Helper functions to initialize the CPU states on systems bootstrapping.

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
    use core::arch::asm;

    let root = root_paddr.as_usize() as u32;

    unsafe {
        // Set TTBR0 and TTBR1 to the same page table
        asm!("mcr p15, 0, {}, c2, c0, 0", in(reg) root); // TTBR0
        asm!("mcr p15, 0, {}, c2, c0, 1", in(reg) root); // TTBR1

        // Set TTBCR to use TTBR0 for all addresses (N=0)
        asm!("mcr p15, 0, {}, c2, c0, 2", in(reg) 0u32);

        // Set Domain Access Control Register (all domains to client mode)
        // Domain 0-15: 01 = Client (check page table permissions)
        asm!("mcr p15, 0, {}, c3, c0, 0", in(reg) 0x55555555u32);

        // Invalidate entire TLB
        crate::asm::flush_tlb(None);

        // Data Synchronization Barrier
        asm!("dsb");

        // Instruction Synchronization Barrier
        asm!("isb");

        // Read SCTLR (System Control Register)
        let mut sctlr: u32;
        asm!("mrc p15, 0, {}, c1, c0, 0", out(reg) sctlr);

        // Enable MMU (M bit), data cache (C bit), instruction cache (I bit)
        sctlr |= (1 << 0) | (1 << 2) | (1 << 12);

        // Write back SCTLR
        asm!("mcr p15, 0, {}, c1, c0, 0", in(reg) sctlr);

        // Synchronization barriers
        asm!("dsb");
        asm!("isb");
    }
}

/// Initializes trap handling on the current CPU.
///
/// In detail, it initializes the exception vector, and sets `TTBR0` to 0 to
/// block low address access.
pub fn init_trap() {
    unsafe extern "C" {
        fn exception_vector_base();
    }
    unsafe {
        crate::asm::write_exception_vector_base(exception_vector_base as *const () as usize);
        crate::asm::write_user_page_table(0.into());
    }
}
