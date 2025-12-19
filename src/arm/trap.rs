//! ARM32 exception handling routines.

use super::TrapFrame;
use crate::trap::PageFaultFlags;

core::arch::global_asm!(include_str!("trap.S"));

/// ARM32 exception types.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum TrapKind {
    /// Reset exception
    Reset = 0,
    /// Undefined instruction exception
    Undefined = 1,
    /// Software interrupt (SVC) exception
    Svc = 2,
    /// Prefetch abort exception
    PrefetchAbort = 3,
    /// Data abort exception
    DataAbort = 4,
    /// Reserved (should never occur)
    Reserved = 5,
    /// IRQ interrupt
    Irq = 6,
    /// FIQ interrupt
    Fiq = 7,
}

/// Handler for invalid/unhandled exceptions.
#[unsafe(no_mangle)]
fn invalid_exception(tf: &TrapFrame, kind: u32) {
    let kind = match kind {
        0 => TrapKind::Reset,
        1 => TrapKind::Undefined,
        2 => TrapKind::Svc,
        3 => TrapKind::PrefetchAbort,
        4 => TrapKind::DataAbort,
        5 => TrapKind::Reserved,
        6 => TrapKind::Irq,
        7 => TrapKind::Fiq,
        _ => TrapKind::Reserved,
    };
    panic!("Invalid exception {:?}:\n{:#x?}", kind, tf);
}

/// Handler for IRQ exceptions.
#[unsafe(no_mangle)]
fn handle_irq_exception(_tf: &TrapFrame) {
    trace!("IRQ received");
    handle_trap!(IRQ, 0);
}

/// Handler for SVC (software interrupt) exceptions.
#[unsafe(no_mangle)]
fn handle_sync_exception(tf: &mut TrapFrame) {
    // SVC instruction encoding: 0xEF000000 | imm24
    // We can read the syscall number from the SVC instruction
    let svc_num = unsafe {
        let pc = tf.pc as *const u32;
        (*pc) & 0x00FFFFFF
    };

    trace!("SVC #{} at {:#x}", svc_num, tf.pc);

    // Handle syscall through the trap handler
    #[cfg(feature = "uspace")]
    {
        crate::trap::handle_syscall(tf, svc_num as usize);
    }
    #[cfg(not(feature = "uspace"))]
    {
        panic!("SVC #{} at {:#x} but uspace feature not enabled:\n{:#x?}", svc_num, tf.pc, tf);
    }
}

/// Handler for prefetch abort exceptions.
#[unsafe(no_mangle)]
fn handle_prefetch_abort_exception(tf: &mut TrapFrame) {
    use core::arch::asm;

    // Read IFSR (Instruction Fault Status Register)
    let ifsr: u32;
    unsafe { asm!("mrc p15, 0, {}, c5, c0, 1", out(reg) ifsr) };

    // Read IFAR (Instruction Fault Address Register)
    let ifar: u32;
    unsafe { asm!("mrc p15, 0, {}, c6, c0, 2", out(reg) ifar) };

    let vaddr = va!(ifar as usize);
    let fs = ifsr & 0xF; // Fault status bits [3:0]

    // Determine if this is a user mode fault
    let is_user = (tf.cpsr & 0x1F) == 0x10; // User mode

    trace!(
        "Prefetch Abort at {:#x}, IFSR={:#x}, IFAR={:#x}, user={}",
        tf.pc, ifsr, ifar, is_user
    );

    // Check if this is a translation/permission fault we can handle
    let mut access_flags = PageFaultFlags::EXECUTE;
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }

    // FS encoding for page faults:
    // 0b0101 (5): Translation fault (Section)
    // 0b0111 (7): Translation fault (Page)
    // 0b1101 (13): Permission fault (Section)
    // 0b1111 (15): Permission fault (Page)
    match fs {
        0b0101 | 0b0111 | 0b1101 | 0b1111 => {
            if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
                panic!(
                    "Unhandled {} Prefetch Abort @ {:#x}, fault_vaddr={:#x}, IFSR={:#x} ({:?}):\n{:#x?}",
                    if is_user { "User" } else { "Supervisor" },
                    tf.pc,
                    vaddr,
                    ifsr,
                    access_flags,
                    tf,
                );
            }
        }
        _ => {
            panic!(
                "Unhandled Prefetch Abort at {:#x}, IFSR={:#x}, IFAR={:#x}:\n{:#x?}",
                tf.pc, ifsr, ifar, tf
            );
        }
    }
}

/// Handler for data abort exceptions.
#[unsafe(no_mangle)]
fn handle_data_abort_exception(tf: &mut TrapFrame) {
    use core::arch::asm;

    // Read DFSR (Data Fault Status Register)
    let dfsr: u32;
    unsafe { asm!("mrc p15, 0, {}, c5, c0, 0", out(reg) dfsr) };

    // Read DFAR (Data Fault Address Register)
    let dfar: u32;
    unsafe { asm!("mrc p15, 0, {}, c6, c0, 0", out(reg) dfar) };

    let vaddr = va!(dfar as usize);
    let fs = dfsr & 0xF; // Fault status bits [3:0]

    // Determine if this is a user mode fault
    let is_user = (tf.cpsr & 0x1F) == 0x10; // User mode

    trace!(
        "Data Abort at {:#x}, DFSR={:#x}, DFAR={:#x}, user={}",
        tf.pc, dfsr, dfar, is_user
    );

    // Determine access type from fault status
    let mut access_flags = PageFaultFlags::empty();

    // Check WnR bit (bit 11) to determine if it's a write
    if (dfsr & (1 << 11)) != 0 {
        access_flags |= PageFaultFlags::WRITE;
    } else {
        access_flags |= PageFaultFlags::READ;
    }

    if is_user {
        access_flags |= PageFaultFlags::USER;
    }

    // FS encoding for page faults:
    // 0b0101 (5): Translation fault (Section)
    // 0b0111 (7): Translation fault (Page)
    // 0b1101 (13): Permission fault (Section)
    // 0b1111 (15): Permission fault (Page)
    match fs {
        0b0101 | 0b0111 | 0b1101 | 0b1111 => {
            if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
                panic!(
                    "Unhandled {} Data Abort @ {:#x}, fault_vaddr={:#x}, DFSR={:#x} ({:?}):\n{:#x?}",
                    if is_user { "User" } else { "Supervisor" },
                    tf.pc,
                    vaddr,
                    dfsr,
                    access_flags,
                    tf,
                );
            }
        }
        _ => {
            panic!(
                "Unhandled Data Abort at {:#x}, DFSR={:#x}, DFAR={:#x}:\n{:#x?}",
                tf.pc, dfsr, dfar, tf
            );
        }
    }
}
