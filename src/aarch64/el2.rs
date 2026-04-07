/// restore guest stack and run.
///
/// need `extern "C" fn handle_vmexit(trap_kind: TrapKind)` to handle the vmexit,
///
/// # Safety
///
/// This function is marked as `naked` to avoid the compiler generating a prologue/epilogue,
#[unsafe(naked)]
pub unsafe extern "C" fn enter_guest() -> ! {
    core::arch::naked_asm!("b __context_vm_entry",);
}
