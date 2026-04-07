use tock_registers::interfaces::Readable;

use super::TrapFrame;
use crate::trap::PageFaultFlags;

#[cfg(not(feature = "arm-el2"))]
core::arch::global_asm!(include_str!("trap.S"));
#[cfg(feature = "arm-el2")]
core::arch::global_asm!(include_str!("trap_el2.S"));

macro_rules! elx {
    ($name:ident.$e:expr) => {{
        #[cfg(not(feature = "arm-el2"))]
        {
            paste::paste! {
                aarch64_cpu::registers::[<$name _EL1>].$e
            }
        }
        #[cfg(feature = "arm-el2")]
        {
            paste::paste! {
                aarch64_cpu::registers::[<$name _EL2>].$e
            }
        }
    }};
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TrapKind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapSource {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[unsafe(no_mangle)]
fn invalid_exception(tf: &TrapFrame, kind: TrapKind, source: TrapSource) {
    panic!(
        "Invalid exception {:?} from {:?}:\n{:#x?}",
        kind, source, tf
    );
}

#[unsafe(no_mangle)]
fn handle_irq_exception(_tf: &TrapFrame) {
    handle_trap!(IRQ, 0);
}

fn handle_instruction_abort(tf: &TrapFrame, iss: u64, is_user: bool) {
    let mut access_flags = PageFaultFlags::EXECUTE;
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    let vaddr = va!(elx!(FAR.get()) as usize);

    // Only handle Translation fault and Permission fault
    if !matches!(iss & 0b111100, 0b0100 | 0b1100) // IFSC or DFSC bits
        || !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user)
    {
        panic!(
            "Unhandled {} Instruction Abort @ {:#x}, fault_vaddr={:#x}, ESR={:#x} ({:?}):\n{:#x?}",
            if is_user { "EL0" } else { "EL1" },
            tf.elr,
            vaddr,
            elx!(ESR.get()),
            access_flags,
            tf,
        );
    }
}

fn handle_data_abort(tf: &TrapFrame, iss: u64, is_user: bool) {
    let wnr = (iss & (1 << 6)) != 0; // WnR: Write not Read
    let cm = (iss & (1 << 8)) != 0; // CM: Cache maintenance
    let mut access_flags = if wnr & !cm {
        PageFaultFlags::WRITE
    } else {
        PageFaultFlags::READ
    };
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    let vaddr = va!(elx!(FAR.get()) as usize);

    // Only handle Translation fault and Permission fault
    if !matches!(iss & 0b111100, 0b0100 | 0b1100) // IFSC or DFSC bits
        || !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user)
    {
        panic!(
            "Unhandled {} Data Abort @ {:#x}, fault_vaddr={:#x}, ESR={:#x} ({:?}):\n{:#x?}",
            if is_user { "EL0" } else { "EL1" },
            tf.elr,
            vaddr,
            elx!(ESR.get()),
            access_flags,
            tf,
        );
    }
}

#[cfg(feature = "arm-el2")]
#[unsafe(no_mangle)]
fn handle_sync_exception(tf: &mut TrapFrame) {
    use aarch64_cpu::registers::ESR_EL2;

    let esr = ESR_EL2.extract();
    let iss = esr.read(ESR_EL2::ISS);
    match esr.read_as_enum(ESR_EL2::EC) {
        Some(ESR_EL2::EC::Value::InstrAbortLowerEL) => handle_instruction_abort(tf, iss, true),
        Some(ESR_EL2::EC::Value::InstrAbortCurrentEL) => handle_instruction_abort(tf, iss, false),
        Some(ESR_EL2::EC::Value::DataAbortLowerEL) => handle_data_abort(tf, iss, true),
        Some(ESR_EL2::EC::Value::DataAbortCurrentEL) => handle_data_abort(tf, iss, false),
        Some(ESR_EL2::EC::Value::Brk64) => {
            debug!("BRK #{:#x} @ {:#x} ", iss, tf.elr);
            tf.elr += 4;
        }
        _ => {
            panic!(
                "Unhandled synchronous exception @ {:#x}: ESR={:#x} (EC {:#08b}, ISS {:#x})",
                tf.elr,
                esr.get(),
                esr.read(ESR_EL2::EC),
                esr.read(ESR_EL2::ISS),
            );
        }
    }
}

#[cfg(not(feature = "arm-el2"))]
#[unsafe(no_mangle)]
fn handle_sync_exception(tf: &mut TrapFrame) {
    use aarch64_cpu::registers::ESR_EL1;

    let esr = ESR_EL1.extract();
    let iss = esr.read(ESR_EL1::ISS);
    match esr.read_as_enum(ESR_EL1::EC) {
        #[cfg(feature = "uspace")]
        Some(ESR_EL1::EC::Value::SVC64) => {
            tf.r[0] = crate::trap::handle_syscall(tf, tf.r[8] as usize) as u64;
        }
        Some(ESR_EL1::EC::Value::InstrAbortLowerEL) => handle_instruction_abort(tf, iss, true),
        Some(ESR_EL1::EC::Value::InstrAbortCurrentEL) => handle_instruction_abort(tf, iss, false),
        Some(ESR_EL1::EC::Value::DataAbortLowerEL) => handle_data_abort(tf, iss, true),
        Some(ESR_EL1::EC::Value::DataAbortCurrentEL) => handle_data_abort(tf, iss, false),
        Some(ESR_EL1::EC::Value::Brk64) => {
            debug!("BRK #{:#x} @ {:#x} ", iss, tf.elr);
            tf.elr += 4;
        }
        _ => {
            panic!(
                "Unhandled synchronous exception @ {:#x}: ESR={:#x} (EC {:#08b}, ISS {:#x})",
                tf.elr,
                esr.get(),
                esr.read(ESR_EL1::EC),
                esr.read(ESR_EL1::ISS),
            );
        }
    }
}
