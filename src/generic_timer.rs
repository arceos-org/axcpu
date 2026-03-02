//! ARM Generic Timer.
//!
//! There are 4 most commonly used timers:
//!
//! - [EL1 Physical Timer][1] (for kernel space use)
//! - [EL1 Virtual Timer][2] (for user space or guest kernel use)
//! - EL2 Physical Timer (for hypervisor use)[^note]
//! - EL3 Physical Timer (for secure monitor use)[^note]
//!
//! [1]: PhysicalTimer
//! [2]: VirtualTimer
//! [^note]: EL2 and EL3 timers are not implemented yet.

/// Common interface for ARM Generic Timer.
pub trait GenericTimer {
    /// Returns the frequency of the timer in Hz.
    fn frequency() -> u32 {
        crate::asm::timer_frequency()
    }

    /// Returns the current value of the timer counter.
    fn counter() -> u64;

    /// Enables or disables the timer.
    fn set_enable(enabled: bool);

    /// Sets the timer to fire an interrupt after the given number of ticks.
    fn set_countdown(ticks: u32);
}

/// The EL1 physical timer.
pub struct PhysicalTimer;

/// The EL1 virtual timer.
pub struct VirtualTimer;

impl GenericTimer for PhysicalTimer {
    fn counter() -> u64 {
        crate::asm::phys_timer_counter()
    }

    fn set_enable(enabled: bool) {
        crate::asm::phys_timer_enable(enabled);
    }

    fn set_countdown(ticks: u32) {
        crate::asm::phys_timer_set_countdown(ticks);
    }
}

impl GenericTimer for VirtualTimer {
    fn counter() -> u64 {
        crate::asm::virt_timer_counter()
    }

    fn set_enable(enabled: bool) {
        crate::asm::virt_timer_enable(enabled);
    }

    fn set_countdown(ticks: u32) {
        crate::asm::virt_timer_set_countdown(ticks);
    }
}
