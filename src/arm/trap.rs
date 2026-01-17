//! ARM32 exception handling routines.

use aarch32_cpu::register::{dfsr::DfsrStatus, ifsr::FsrStatus};

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
        panic!(
            "SVC #{} at {:#x} but uspace feature not enabled:\n{:#x?}",
            svc_num, tf.pc, tf
        );
    }
}

/// Handler for prefetch abort exceptions.
#[unsafe(no_mangle)]
fn handle_prefetch_abort_exception(tf: &mut TrapFrame) {
    let (fsr, far) = (super::asm::read_ifsr(), super::asm::read_ifar());

    let cpsr = super::asm::read_cpsr();

    let is_user = cpsr.mode() == Ok(aarch32_cpu::register::cpsr::ProcessorMode::Usr);

    let access_flags = PageFaultFlags::EXECUTE
        | if is_user {
            PageFaultFlags::USER
        } else {
            PageFaultFlags::empty()
        };

    let fsr_status = match fsr.status() {
        Ok(status) => status,
        Err(raw) => panic!(
            "Unknown IFSR status {:#x} in Prefetch Abort at {:#x}:\n{:#x?}",
            raw, tf.pc, tf
        ),
    };

    match fsr_status {
        FsrStatus::TranslationFaultFirstLevel | FsrStatus::TranslationFaultSecondLevel => {
            let vaddr = va!(far.0 as usize);
            if handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
                return;
            }
        }
        _ => {}
    }
}

/// Handler for data abort exceptions.
#[unsafe(no_mangle)]
fn handle_data_abort_exception(tf: &mut TrapFrame) {
    let (fsr, far) = (super::asm::read_dfsr(), super::asm::read_dfar());

    let cpsr = super::asm::read_cpsr();

    let is_user = cpsr.mode() == Ok(aarch32_cpu::register::cpsr::ProcessorMode::Usr);

    let mut access_flags = if fsr.wnr() {
        PageFaultFlags::WRITE
    } else {
        PageFaultFlags::READ
    };

    if is_user {
        access_flags |= PageFaultFlags::USER;
    }

    let fsr_status = match fsr.status() {
        Ok(status) => status,
        Err(raw) => panic!(
            "Unknown DFSR status {:#x} in Data Abort at {:#x}:\n{:#x?}",
            raw, tf.pc, tf
        ),
    };

    match fsr_status {
        DfsrStatus::CommonFsr(FsrStatus::TranslationFaultFirstLevel)
        | DfsrStatus::CommonFsr(FsrStatus::TranslationFaultSecondLevel)
        | DfsrStatus::CommonFsr(FsrStatus::PermissionFaultFirstLevel)
        | DfsrStatus::CommonFsr(FsrStatus::PermissionFaultSecondLevel) => {
            let vaddr = va!(far.0 as usize);
            if handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
                return;
            }
        }
        _ => {}
    }
}
