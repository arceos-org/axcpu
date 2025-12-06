use super::TrapFrame;
use crate::trap::PageFaultFlags;

core::arch::global_asm!(include_str!("trap.S"));

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapKind {
    Reset = 0,
    Undefined = 1,
    Svc = 2,
    PrefetchAbort = 3,
    DataAbort = 4,
    Reserved = 5,
    Irq = 6,
    Fiq = 7,
}

#[unsafe(no_mangle)]
fn invalid_exception(tf: &TrapFrame, kind: TrapKind) {
    panic!("Invalid exception {:?}:\n{:#x?}", kind, tf);
}

#[unsafe(no_mangle)]
fn handle_irq_exception(_tf: &TrapFrame) {
    handle_trap!(IRQ, 0);
}

#[unsafe(no_mangle)]
fn handle_sync_exception(tf: &mut TrapFrame) {
    // Read ESR (Exception Syndrome Register) - actually FSR for ARMv7-A
    // We need to check if it's a data abort or prefetch abort
    let pc = tf.pc;
    
    // Check CPSR mode bits to determine exception type
    let mode = tf.cpsr & 0x1F;
    
    match mode {
        0x13 => {
            // SVC mode - could be from various exceptions
            // Try to determine from context
            handle_data_abort(tf, false);
        }
        0x17 => {
            // Abort mode
            handle_data_abort(tf, false);
        }
        _ => {
            panic!("Unhandled sync exception at {:#x}, mode {:#x}:\n{:#x?}", pc, mode, tf);
        }
    }
}

fn handle_data_abort(tf: &TrapFrame, is_user: bool) {
    use core::arch::asm;
    
    // Read DFSR (Data Fault Status Register)
    let dfsr: u32;
    unsafe { asm!("mrc p15, 0, {}, c5, c0, 0", out(reg) dfsr) };
    
    // Read DFAR (Data Fault Address Register)
    let dfar: u32;
    unsafe { asm!("mrc p15, 0, {}, c6, c0, 0", out(reg) dfar) };
    
    let vaddr = va!(dfar as usize);
    let fs = dfsr & 0xF; // Fault status bits [3:0]
    
    // Determine access type from fault status
    let mut access_flags = PageFaultFlags::empty();
    
    // FS encoding for page faults:
    // 0b0101, 0b0111: Translation fault
    // 0b1101, 0b1111: Permission fault
    match fs {
        0b0101 | 0b0111 | 0b1101 | 0b1111 => {
            // Check WnR bit (bit 11) to determine if it's a write
            if (dfsr & (1 << 11)) != 0 {
                access_flags |= PageFaultFlags::WRITE;
            } else {
                access_flags |= PageFaultFlags::READ;
            }
        }
        _ => {
            panic!(
                "Unhandled data abort at {:#x}, DFSR={:#x}, DFAR={:#x}:\n{:#x?}",
                tf.pc, dfsr, dfar, tf
            );
        }
    }
    
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    
    if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
        panic!(
            "Unhandled {} Data Abort @ {:#x}, fault_vaddr={:#x} ({:?}):\n{:#x?}",
            if is_user { "User" } else { "Supervisor" },
            tf.pc,
            vaddr,
            access_flags,
            tf,
        );
    }
}

fn handle_prefetch_abort(tf: &TrapFrame, is_user: bool) {
    use core::arch::asm;
    
    // Read IFSR (Instruction Fault Status Register)
    let ifsr: u32;
    unsafe { asm!("mrc p15, 0, {}, c5, c0, 1", out(reg) ifsr) };
    
    // Read IFAR (Instruction Fault Address Register)
    let ifar: u32;
    unsafe { asm!("mrc p15, 0, {}, c6, c0, 2", out(reg) ifar) };
    
    let vaddr = va!(ifar as usize);
    let mut access_flags = PageFaultFlags::EXECUTE;
    
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    
    let fs = ifsr & 0xF;
    if !matches!(fs, 0b0101 | 0b0111 | 0b1101 | 0b1111) {
        panic!(
            "Unhandled prefetch abort at {:#x}, IFSR={:#x}, IFAR={:#x}:\n{:#x?}",
            tf.pc, ifsr, ifar, tf
        );
    }
    
    if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
        panic!(
            "Unhandled {} Prefetch Abort @ {:#x}, fault_vaddr={:#x} ({:?}):\n{:#x?}",
            if is_user { "User" } else { "Supervisor" },
            tf.pc,
            vaddr,
            access_flags,
            tf,
        );
    }
}
