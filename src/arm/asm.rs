//! Wrapper functions for assembly instructions.

use core::arch::asm;
use memory_addr::{PhysAddr, VirtAddr};

use aarch32_cpu::register::*;

pub use aarch32_cpu::asm::{dmb, dsb, isb, sev, wfe, wfi};

/// Allows the current CPU to respond to interrupts.
#[inline]
pub fn enable_irqs() {
    unsafe { aarch32_cpu::interrupt::enable() };
}

/// Makes the current CPU to ignore interrupts.
#[inline]
pub fn disable_irqs() {
    aarch32_cpu::interrupt::disable();
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In ARMv7-A, it checks the I bit in the CPSR.
#[inline]
pub fn irqs_enabled() -> bool {
    let cpsr = Cpsr::read();
    !cpsr.i() // I bit is 1 when disabled, 0 when enabled
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn wait_for_irqs() {
    wfi();
}

/// Halt the current CPU.
#[inline]
pub fn halt() {
    disable_irqs();
    wfi(); // should never return
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
        dsb();
        isb();
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
        dsb();
        isb();
    }
}

/// Writes the Translation Table Base Control Register (`TTBCR`).
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_ttbcr(ttbcr: u32) {
    unsafe {
        asm!("mcr p15, 0, {}, c2, c0, 2", in(reg) ttbcr);
        dsb();
        isb();
    }
}

/// Writes the Domain Access Control Register (`DACR`).
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_dacr(dacr: u32) {
    unsafe {
        asm!("mcr p15, 0, {}, c3, c0, 0", in(reg) dacr);
        dsb();
        isb();
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
            TlbIAll::write();
        }
        dsb();
        isb();
    }
}

/// Flushes the entire instruction cache.
#[inline]
pub fn flush_icache_all() {
    unsafe {
        // ICIALLU - Instruction Cache Invalidate All to PoU
        asm!("mcr p15, 0, {}, c7, c5, 0", in(reg) 0);
        dsb();
        isb();
    }
}

/// Flushes the data cache line at the given virtual address
#[inline]
pub fn flush_dcache_line(vaddr: VirtAddr) {
    let addr = vaddr.as_usize() as u32;
    aarch32_cpu::cache::clean_and_invalidate_data_cache_line_to_poc(addr);
    dsb();
    isb();
}

/// Reads the exception vector base address register (`VBAR`).
#[inline]
pub fn read_exception_vector_base() -> usize {
    // VBAR: CP15, c12, CRn=12, CRm=0, Op1=0, Op2=0
    let vbar: u32;
    unsafe { asm!("mrc p15, 0, {}, c12, c0, 0", out(reg) vbar) };
    vbar as usize
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
    dsb();
    isb();
}

/// Enable FP/SIMD instructions by setting the appropriate bits in CPACR.
#[cfg(feature = "fp-simd")]
#[inline]
pub fn enable_fp() {
    let mut cpacr = Cpacr::read();
    // Enable CP10 and CP11 (VFP/NEON)
    cpacr.0 |= (0b11 << 20) | (0b11 << 22);
    unsafe {
        // Write CPACR
        Cpacr::write(cpacr);
        isb();
        // Enable VFP by setting EN bit in FPEXC
        asm!("vmsr fpexc, {}", in(reg) 0x40000000u32);
    }
}

/// Reads the Data Fault Status Register (DFSR).
#[inline]
pub fn read_dfsr() -> Dfsr {
    Dfsr::read()
}

/// Reads the Data Fault Address Register (DFAR).
#[inline]
pub fn read_dfar() -> Dfar {
    Dfar::read()
}

/// Reads the Instruction Fault Status Register (IFSR).
#[inline]
pub fn read_ifsr() -> Ifsr {
    Ifsr::read()
}

/// Reads the Instruction Fault Address Register (IFAR).
#[inline]
pub fn read_ifar() -> Ifar {
    Ifar::read()
}

/// Reads the System Control Register (SCTLR).
#[inline]
pub fn read_sctlr() -> Sctlr {
    Sctlr::read()
}

/// Writes the System Control Register (SCTLR).
///
/// # Safety
///
/// This function is unsafe as it can modify critical system settings.
#[inline]
pub unsafe fn write_sctlr(sctlr: Sctlr) {
    Sctlr::write(sctlr);
    dsb();
    isb();
}

/// Reads the CPSR (Current Program Status Register).
#[inline]
pub fn read_cpsr() -> Cpsr {
    Cpsr::read()
}

/// Reads the timer counter (CNTPCT)
#[inline]
pub fn timer_counter() -> u64 {
    let mut low: u32;
    let mut high: u32;
    // mrrc p15, 0, <Rt>, <Rt2>, c14
    unsafe {
        asm!("mrrc p15, 0, {}, {}, c14", out(reg) low, out(reg) high);
    }
    ((high as u64) << 32) | (low as u64)
}

/// Reads the timer frequency (CNTFRQ)
#[inline]
pub fn timer_frequency() -> u64 {
    let freq: u32;
    // mrc p15, 0, <Rt>, c14, c0, 0
    unsafe {
        asm!("mrc p15, 0, {}, c14, c0, 0", out(reg) freq);
    }
    freq as u64
}

/// Writes the timer compare value (CNTP_CVAL)
#[inline]
pub fn write_timer_comparevalue(val: u64) {
    let low = (val & 0xFFFFFFFF) as u32;
    let high = (val >> 32) as u32;
    // mcrr p15, 2, <Rt>, <Rt2>, c14 (CNTP_CVAL)
    unsafe {
        asm!("mcrr p15, 2, {}, {}, c14", in(reg) low, in(reg) high);
    }
}

/// Writes the timer control register (CNTP_CTL)
#[inline]
pub fn write_timer_control(val: u32) {
    // mcr p15, 0, <Rt>, c14, c2, 1
    unsafe {
        asm!("mcr p15, 0, {}, c14, c2, 1", in(reg) val);
    }
}
