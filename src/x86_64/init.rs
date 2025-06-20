//! Helper functions to initialize the CPU states on systems bootstrapping.

pub use super::gdt::init_gdt;
pub use super::idt::init_idt;

#[cfg(feature = "uspace")]
pub use super::syscall::init_syscall;

/// Initializes the per-CPU data structures.
///
/// It calls the initialization function of the [`percpu`] crate. It (or other
/// alternative initialization) should be called before [`init_trap`].
///
/// [`percpu`]: https://docs.rs/percpu/latest/percpu/index.html
pub fn init_percpu(cpu_id: usize) {
    percpu::init();
    percpu::init_percpu_reg(cpu_id);
}

/// Initializes trap handling on the current CPU.
///
/// In detail, it initializes the GDT, IDT on x86_64 platforms ([`init_gdt`] and
/// [`init_idt`]). If the `uspace` feature is enabled, it also initializes
/// relevant model-specific registers to configure the handler for `syscall`
/// instruction ([`init_syscall`]).
///
/// # Notes
/// Before calling this function, the initialization function of the [`percpu`] crate
/// should have been invoked to ensure that the per-CPU data structures are set up
/// correctly (i.e., by calling [`init_percpu`]).
///
/// [`percpu`]: https://docs.rs/percpu/latest/percpu/index.html
pub fn init_trap() {
    init_gdt();
    init_idt();
    #[cfg(feature = "uspace")]
    init_syscall();
}
