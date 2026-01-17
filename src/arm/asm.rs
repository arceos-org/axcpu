//! Wrapper functions for assembly instructions.

use core::arch::asm;
use memory_addr::{PhysAddr, VirtAddr};

/// Allows the current CPU to respond to interrupts.
///
/// In ARMv7-A, it unmasks IRQs by clearing the I bit in the CPSR.
#[inline]
pub fn enable_irqs() {
    aarch32_cpu::asm::irq_enable();
}

/// Makes the current CPU to ignore interrupts.
///
/// In ARMv7-A, it masks IRQs by setting the I bit in the CPSR.
#[inline]
pub fn disable_irqs() {
    aarch32_cpu::asm::irq_disable();
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In ARMv7-A, it checks the I bit in the CPSR.
#[inline]
pub fn irqs_enabled() -> bool {
    let cpsr = aarch32_cpu::register::Cpsr::read();
    !cpsr.i() // I bit is 1 when disabled, 0 when enabled
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn wait_for_irqs() {
    aarch32_cpu::asm::wfi();
}

/// Halt the current CPU.
#[inline]
pub fn halt() {
    disable_irqs();
    aarch32_cpu::asm::wfi(); // should never return
}

/// Reads the current page table root register for kernel space (`TTBR1`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_kernel_page_table() -> PhysAddr {
    // TTBR1: CP15, c2, CRn=2, CRm=0, Op1=0, Op2=1
    let root: u32;
    unsafe { asm!("mrc p15, 0, {}, c2, c0, 1", out(reg) root) };
    pa!(root as usize)
}

/// Reads the current page table root register for user space (`TTBR0`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_user_page_table() -> PhysAddr {
    // TTBR0: CP15, c2, CRn=2, CRm=0, Op1=0, Op2=0
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
        aarch32_cpu::asm::dsb();
        aarch32_cpu::asm::isb();
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
        aarch32_cpu::asm::dsb();
        aarch32_cpu::asm::isb();
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
            aarch32_cpu::register::TlbIAll::write();
        }
        aarch32_cpu::asm::dsb();
        aarch32_cpu::asm::isb();
    }
}

/// Flushes the entire instruction cache.
#[inline]
pub fn flush_icache_all() {
    unsafe {
        // ICIALLU - Instruction Cache Invalidate All to PoU
        asm!("mcr p15, 0, {}, c7, c5, 0", in(reg) 0);
        aarch32_cpu::asm::dsb();
        aarch32_cpu::asm::isb();
    }
}

/// Flushes the data cache line at the given virtual address
#[inline]
pub fn flush_dcache_line(vaddr: VirtAddr) {
    let addr = vaddr.as_usize() as u32;
    aarch32_cpu::cache::clean_and_invalidate_data_cache_line_to_poc(addr);
    aarch32_cpu::asm::dsb();
    aarch32_cpu::asm::isb();
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
    asm!("mcr p15, 0, {}, c12, c0, 0", in(reg) vbar);
    aarch32_cpu::asm::dsb();
    aarch32_cpu::asm::isb();
}

/// Enable FP/SIMD instructions by setting the appropriate bits in CPACR.
#[cfg(feature = "fp-simd")]
#[inline]
pub fn enable_fp() {
    let mut cpacr = aarch32_cpu::register::Cpacr::read();
    // Enable CP10 and CP11 (VFP/NEON)
    cpacr.0 |= (0b11 << 20) | (0b11 << 22);
    unsafe {
        aarch32_cpu::register::Cpacr::write(cpacr);
    }
    aarch32_cpu::asm::isb();
    // Enable VFP by setting EN bit in FPEXC
    unsafe {
        asm!("vmsr fpexc, {}", in(reg) 0x40000000u32);
    }
}

/// Reads the exception vector base address register (`VBAR`).
#[inline]
pub fn read_exception_vector_base() -> usize {
    // VBAR: CP15, c12, CRn=12, CRm=0, Op1=0, Op2=0
    let vbar: u32;
    unsafe { asm!("mrc p15, 0, {}, c12, c0, 0", out(reg) vbar) };
    vbar as usize
}

/// Reads the Data Fault Status Register (DFSR).
#[inline]
pub fn read_dfsr() -> aarch32_cpu::register::Dfsr {
    aarch32_cpu::register::Dfsr::read()
}

/// Reads the Data Fault Address Register (DFAR).
#[inline]
pub fn read_dfar() -> aarch32_cpu::register::Dfar {
    aarch32_cpu::register::Dfar::read()
}

/// Reads the Instruction Fault Status Register (IFSR).
#[inline]
pub fn read_ifsr() -> aarch32_cpu::register::Ifsr {
    aarch32_cpu::register::Ifsr::read()
}

/// Reads the Instruction Fault Address Register (IFAR).
#[inline]
pub fn read_ifar() -> aarch32_cpu::register::Ifar {
    aarch32_cpu::register::Ifar::read()
}

/// Reads the System Control Register (SCTLR).
#[inline]
pub fn read_sctlr() -> aarch32_cpu::register::Sctlr {
    aarch32_cpu::register::Sctlr::read()
}

/// Writes the System Control Register (SCTLR).
///
/// # Safety
///
/// This function is unsafe as it can modify critical system settings.
#[inline]
pub unsafe fn write_sctlr(sctlr: aarch32_cpu::register::Sctlr) {
    aarch32_cpu::register::Sctlr::write(sctlr);
    aarch32_cpu::asm::dsb();
    aarch32_cpu::asm::isb();
}

/// Reads the CPSR (Current Program Status Register).
#[inline]
pub fn read_cpsr() -> aarch32_cpu::register::Cpsr {
    aarch32_cpu::register::Cpsr::read()
}

/// Data Synchronization Barrier.
#[inline]
pub fn dsb() {
    aarch32_cpu::asm::dsb();
}

/// Data Memory Barrier.
#[inline]
pub fn dmb() {
    aarch32_cpu::asm::dmb();
}

/// Instruction Synchronization Barrier.
#[inline]
pub fn isb() {
    aarch32_cpu::asm::isb();
}

/// Send Event - wake up cores waiting in WFE.
#[inline]
pub fn sev() {
    aarch32_cpu::asm::sev();
}

/// Wait for Event.
#[inline]
pub fn wfe() {
    aarch32_cpu::asm::wfe();
}
