use aarch64_cpu::registers::*;

use crate::{TrapFrame, TrapKind};

/// Macro to save the host function context to the stack.
///
/// This macro saves the values of the callee-saved registers (`x19` to `x30`) to the stack.
/// The stack pointer (`sp`) is adjusted accordingly
/// to make space for the saved registers.
///
/// ## Note
///
/// This macro should be used in conjunction with `restore_regs_from_stack!` to ensure that
/// the saved registers are properly restored when needed,
/// and the control flow can be returned to `Aarch64VCpu.run()` in `vcpu.rs` happily.
macro_rules! save_regs_to_stack {
    () => {
        "
        sub     sp, sp, 12 * 8
        stp     x29, x30, [sp, 10 * 8]
        stp     x27, x28, [sp, 8 * 8]
        stp     x25, x26, [sp, 6 * 8]
        stp     x23, x24, [sp, 4 * 8]
        stp     x21, x22, [sp, 2 * 8]
        stp     x19, x20, [sp]"
    };
}

/// Macro to restore the host function context from the stack.
///
/// This macro restores the values of the callee-saved general-purpose registers (`x19` to `x30`) from the stack.
/// The stack pointer (`sp`) is adjusted back after restoring the registers.
///
/// ## Note
///
/// This macro is called in `return_run_guest()` in exception.rs,
/// it should only be used after `save_regs_to_stack!` to correctly restore the control flow of `Aarch64VCpu.run()`.
macro_rules! restore_regs_from_stack {
    () => {
        "
        ldp     x19, x20, [sp]
        ldp     x21, x22, [sp, 2 * 8]
        ldp     x23, x24, [sp, 4 * 8]
        ldp     x25, x26, [sp, 6 * 8]
        ldp     x27, x28, [sp, 8 * 8]
        ldp     x29, x30, [sp, 10 * 8]
        add     sp, sp, 12 * 8"
    };
}

/// # Safety
///
/// This function is unsafe because it changes processor mode and the stack.
pub unsafe fn enter_geust() -> TrapKind {
    match __run_guest() {
        0 => TrapKind::Synchronous,
        1 => TrapKind::Irq,
        2 => TrapKind::Fiq,
        3 => TrapKind::SError,
        _ => unreachable!(),
    }
}

/// Save host context and run guest.
///
/// When a VM-Exit happens when guest's vCpu is running,
/// the control flow will be redirected to this function through `return_run_guest`.
#[unsafe(naked)]
unsafe extern "C" fn __run_guest() -> usize {
    // Fixes: https://github.com/arceos-hypervisor/arm_vcpu/issues/22
    //
    // The original issue seems to be caused by an unexpected compiler optimization that takes
    // the dummy return value `0` of `run_guest` as the actual return value. By replacing the
    // original `run_guest` with the current naked one, we eliminate the dummy code path of the
    // original version, and ensure that the compiler does not perform any unexpected return
    // value optimization.
    core::arch::naked_asm!(
        // Save host context.
        save_regs_to_stack!(),
        // Save current host stack top to `self.host_stack_top`.
        //
        // 'extern "C"' here specifies the aapcs64 calling convention, according to which
        // the first and only parameter, the pointer of self, should be in x0:
        "mov x9, sp",
        "add x0, x0, {host_stack_top_offset}",
        "str x9, [x0]",
        // Go to `context_vm_entry`.
        "b context_vm_entry",
        // Panic if the control flow comes back here, which should never happen.
        "b {run_guest_panic}",
        host_stack_top_offset = const core::mem::size_of::<TrapFrame>(),
        run_guest_panic = sym run_guest_panic,
    );
}

/// This function is called when the control flow comes back to `run_guest`. To provide a error
/// message for debugging purposes.
///
/// This function may fail as the stack may have been corrupted when this function is called.
/// But we won't handle it here for now.
unsafe fn run_guest_panic() -> ! {
    panic!("run_guest_panic");
}

/// A trampoline function for sp switching during handling VM exits,
/// when **there is a active VCPU running**, which means that the host context is stored
/// into host stack in `run_guest` function.
///
/// # Functionality
///
/// 1. **Restore Previous Host Stack pointor:**
///     - The guest context frame is aleady saved by `SAVE_REGS_FROM_EL1` macro in exception.S.
///       This function firstly adjusts the `sp` to skip the exception frame
///       (adding `34 * 8` to the stack pointer) according to the memory layout of `Aarch64VCpu` struct,
///       which makes current `sp` point to the address of `host_stack_top`.
///       The host stack top value is restored by `ldr`.
///
/// 2. **Restore Host Context:**
///     - The `restore_regs_from_stack!()` macro is invoked to restore the host function context
///       from the stack. This macro handles the restoration of the host's callee-saved general-purpose
///       registers (`x19` to `x30`).
///
/// 3. **Restore Host Control Flow:**
///     - The `ret` instruction is used to return control to the host context after
///       the guest context has been saved in `Aarch64VCpu` struct and the host context restored.
///       Finally the control flow is returned back to `Aarch64VCpu.run()` in [vcpu.rs].
///
/// # Notes
///
/// - This function is typically invoked when a VM exit occurs, requiring the
///   hypervisor to switch context from the guest to the host. The precise control
///   over stack and register management ensures that the transition is smooth and
///   that the host can correctly resume execution.
///
/// - The `options(noreturn)` directive indicates that this function will not return
///   to its caller, as control will be transferred back to the host context via `ret`.
///
/// - This function is not typically called directly from Rust code. Instead, it is
///   invoked as part of the low-level hypervisor or VM exit handling routines.
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn vmexit_trampoline() -> ! {
    core::arch::naked_asm!(
        // Curretly `sp` points to the base address of `Aarch64VCpu.ctx`, which stores guest's `TrapFrame`.
        "add x9, sp, 34 * 8", // Skip the exception frame.
        // Currently `x9` points to `&Aarch64VCpu.host_stack_top`, see `run_guest()` in vcpu.rs.
        "ldr x10, [x9]", // Get `host_stack_top` value from `&Aarch64VCpu.host_stack_top`.
        "mov sp, x10",   // Set `sp` as the host stack top.
        restore_regs_from_stack!(), // Restore host function context frame.
        "ret", // Control flow is handed back to Aarch64VCpu.run(), simulating the normal return of the `run_guest` function.
    )
}
