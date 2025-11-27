// RTOS backend implementations
//
// Currently supported:
// - RT-Linux (PREEMPT_RT) - Full implementation for Linux systems
//
// Future backends (contributions welcome):
// - FreeRTOS - Embedded RTOS
// - Zephyr - Modern scalable RTOS
// - QNX Neutrino - Commercial hard real-time OS

pub mod rtlinux;

pub use rtlinux::RTLinuxBackend;
