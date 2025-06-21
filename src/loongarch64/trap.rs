use loongArch64::register::{
    badv,
    estat::{self, Exception, Trap},
};

use super::context::TrapFrame;
use crate::trap::PageFaultFlags;

core::arch::global_asm!(
    include_asm_macros!(),
    include_str!("trap.S"),
    trapframe_size = const (core::mem::size_of::<TrapFrame>()),
);

fn handle_breakpoint(era: &mut usize) {
    debug!("Exception(Breakpoint) @ {:#x} ", era);
    *era += 4;
}

fn handle_page_fault(tf: &TrapFrame, mut access_flags: PageFaultFlags, is_user: bool) {
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    let vaddr = va!(badv::read().raw());
    if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
        panic!(
            "Unhandled {} Page Fault @ {:#x}, fault_vaddr={:#x} ({:?}):\n{:#x?}",
            if is_user { "PLV3" } else { "PLV0" },
            tf.era,
            vaddr,
            access_flags,
            tf,
        );
    }
}

#[unsafe(no_mangle)]
fn loongarch64_trap_handler(tf: &mut TrapFrame, from_user: bool) {
    let estat = estat::read();
    let trap = estat.cause();

    if matches!(trap, Trap::Exception(_)) {
        unmask_irqs(tf);
    }

    match trap {
        #[cfg(feature = "uspace")]
        Trap::Exception(Exception::Syscall) => {
            tf.regs.a0 = crate::trap::handle_syscall(tf, tf.regs.a7) as usize;
            tf.era += 4;
        }
        Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::PageNonReadableFault) => {
            handle_page_fault(tf, PageFaultFlags::READ, from_user)
        }
        Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::PageModifyFault) => {
            handle_page_fault(tf, PageFaultFlags::WRITE, from_user)
        }
        Trap::Exception(Exception::FetchPageFault)
        | Trap::Exception(Exception::PageNonExecutableFault) => {
            handle_page_fault(tf, PageFaultFlags::EXECUTE, from_user);
        }
        Trap::Exception(Exception::Breakpoint) => handle_breakpoint(&mut tf.era),
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            handle_trap!(IRQ, irq_num);
        }
        _ => {
            panic!(
                "Unhandled trap {:?} @ {:#x}:\n{:#x?}",
                estat.cause(),
                tf.era,
                tf
            );
        }
    }
    mask_irqs();
}

// Interrupt unmasking function for exception handling.
// NOTE: It must be invoked after the switch to kernel mode has finished
//
// If interrupts were enabled before the exception (`PIE` bit in `PRMD` is set),
// re-enable interrupts before handling the exception.
//
// On loongarch64, when an exception is triggered, records the old value of
// `IE` domain in `CSR.CRMD` in `PIE` domain in `PRMD`. When the `ERTN`
// instruction is executed to return from the exception handler, the hardware
// restores the value of the `PIE` domain to the `IE` domain of `CSR.CRMD`.
fn unmask_irqs(tf: &TrapFrame) {
    const PIE: usize = 1 << 2;
    if tf.prmd & PIE == PIE {
        crate::asm::enable_irqs();
    } else {
        debug!("Interrupts were disabled before exception");
    }
}

fn mask_irqs() {
    crate::asm::disable_irqs();
}
