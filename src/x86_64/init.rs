//! Helper functions to initialize the CPU states on systems bootstrapping.

/// Initializes trap handling on the current CPU.
///
/// In detail, it initializes the GDT, IDT on x86_64 platforms. If the `uspace`
/// feature is enabled, it also initializes relevant model-specific registers to
/// configure the handler for `syscall` instruction.
///
/// # Notes
/// Before calling this function, the initialization function of the [`percpu`]
/// crate should have been invoked to ensure that the per-CPU data structures
/// are set up correctly.
///
/// [`percpu`]: https://docs.rs/percpu/latest/percpu/index.html
pub fn init_trap() {
    super::gdt::init();
    super::idt::init();
    #[cfg(feature = "uspace")]
    super::syscall::init_syscall();
}
