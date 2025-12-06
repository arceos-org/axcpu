//! Wrapper functions for assembly instructions.

use core::arch::asm;
use memory_addr::{PhysAddr, VirtAddr};

/// Allows the current CPU to respond to interrupts.
///
/// In ARMv7-A, it unmasks IRQs by clearing the I bit in the CPSR.
#[inline]
pub fn enable_irqs() {
    unsafe { asm!("cpsie i") };
}

/// Makes the current CPU to ignore interrupts.
///
/// In ARMv7-A, it masks IRQs by setting the I bit in the CPSR.
#[inline]
pub fn disable_irqs() {
    unsafe { asm!("cpsid i") };
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In ARMv7-A, it checks the I bit in the CPSR.
#[inline]
pub fn irqs_enabled() -> bool {
    let cpsr: u32;
    unsafe { asm!("mrs {}, cpsr", out(reg) cpsr) };
    (cpsr & (1 << 7)) == 0 // I bit is bit 7, 0 means enabled
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn wait_for_irqs() {
    unsafe { asm!("wfi") };
}

/// Halt the current CPU.
#[inline]
pub fn halt() {
    disable_irqs();
    unsafe { asm!("wfi") }; // should never return
}

/// Reads the current page table root register for kernel space (`TTBR1`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_kernel_page_table() -> PhysAddr {
    let root: u32;
    unsafe { asm!("mrc p15, 0, {}, c2, c0, 1", out(reg) root) };
    pa!(root as usize)
}

/// Reads the current page table root register for user space (`TTBR0`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_user_page_table() -> PhysAddr {
    let root: u32;
    unsafe { asm!("mrc p15, 0, {}, c2, c0, 0", out(reg) root) };
    pa!(root as usize)
}

/// Writes the register to update the current page table root for kernel space
/// (`TTBR1`).
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_kernel_page_table(root_paddr: PhysAddr) {
    let root = root_paddr.as_usize() as u32;
    unsafe {
        asm!("mcr p15, 0, {}, c2, c0, 1", in(reg) root);
        asm!("dsb");
        asm!("isb");
    }
}

/// Writes the register to update the current page table root for user space
/// (`TTBR0`).
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_user_page_table(root_paddr: PhysAddr) {
    let root = root_paddr.as_usize() as u32;
    unsafe {
        asm!("mcr p15, 0, {}, c2, c0, 0", in(reg) root);
        asm!("dsb");
        asm!("isb");
    }
}

/// Flushes the TLB.
///
/// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
/// entry that maps the given virtual address.
#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    unsafe {
        if let Some(vaddr) = vaddr {
            let addr = vaddr.as_usize() as u32;
            // TLBIMVA - TLB Invalidate by MVA
            asm!("mcr p15, 0, {}, c8, c7, 1", in(reg) addr);
        } else {
            // TLBIALL - TLB Invalidate All
            asm!("mcr p15, 0, {}, c8, c7, 0", in(reg) 0);
        }
        asm!("dsb");
        asm!("isb");
    }
}

/// Flushes the entire instruction cache.
#[inline]
pub fn flush_icache_all() {
    unsafe {
        asm!("mcr p15, 0, {}, c7, c5, 0", in(reg) 0); // ICIALLU
        asm!("dsb");
        asm!("isb");
    }
}

/// Flushes the data cache line at the given virtual address
#[inline]
pub fn flush_dcache_line(vaddr: VirtAddr) {
    let addr = vaddr.as_usize() as u32;
    unsafe {
        asm!("mcr p15, 0, {}, c7, c14, 1", in(reg) addr); // DCCIMVAC
        asm!("dsb");
        asm!("isb");
    }
}

/// Writes exception vector base address register (`VBAR`).
///
/// # Safety
///
/// This function is unsafe as it changes the exception handling behavior of the
/// current CPU.
#[inline]
pub unsafe fn write_exception_vector_base(vbar: usize) {
    let vbar = vbar as u32;
    unsafe {
        asm!("mcr p15, 0, {}, c12, c0, 0", in(reg) vbar);
        asm!("dsb");
        asm!("isb");
    }
}

/// Enable FP/SIMD instructions by setting the appropriate bits in CPACR.
#[cfg(feature = "fp-simd")]
#[inline]
pub fn enable_fp() {
    unsafe {
        let mut cpacr: u32;
        asm!("mrc p15, 0, {}, c1, c0, 2", out(reg) cpacr);
        // Enable CP10 and CP11 (VFP/NEON)
        cpacr |= (0b11 << 20) | (0b11 << 22);
        asm!("mcr p15, 0, {}, c1, c0, 2", in(reg) cpacr);
        asm!("isb");
        // Enable VFP by setting EN bit in FPEXC
        asm!("vmsr fpexc, {}", in(reg) 0x40000000u32);
    }
}
