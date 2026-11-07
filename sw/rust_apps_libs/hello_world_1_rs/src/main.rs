//! A9#1 application: no_std Rust port of `hello_world_1/src/helloworld1.c`.
//!
//! Waits for A9#0 to publish data through shared DDR, acknowledges it, then
//! sends a message every 3 seconds.
//!
//! The cores don't share peripherals: A9#0 owns UART0, so this core never
//! prints. Its messages go through the shared-memory mailbox, and A9#0 prints
//! them. Its `sleep()` uses its own SCU private timer (the BSP's
//! `XILTIMER_sleep_timer`, set in sw/build.py), since A9#0 owns the global timer.
//!
//! `init_platform()`/`cleanup_platform()` from the C version's platform.c do
//! nothing on the A9, so they have no equivalent here.
#![no_std]
#![no_main]

use core::ffi::c_int;
use core::hint::black_box;
use core::sync::atomic::Ordering;

use shared_mem::{CORE0_PRIVATE_BASE, CORE0_PRIVATE_END, sections, shared};
use xil_a9_1_sys::*;

/// Sends a message to A9#0, which prints it
fn post(msg: &str) {
    // SAFETY: A9#1 is the mailbox's only producer, and A9#0 cleared the mailbox
    // before waking this core
    unsafe { shared().post(msg.as_bytes()) }
}

/// Called by the BSP's startup code (xil-crt0.S) after it sets up the MMU,
/// caches, stacks and .bss
#[unsafe(no_mangle)]
extern "C" fn main() -> c_int {
    let shared = shared();

    // Fault on any access from this core to A9#0's private DDR region (the shared section stays mapped)
    for addr in sections(CORE0_PRIVATE_BASE, CORE0_PRIVATE_END) {
        unsafe { Xil_SetTlbAttributes(addr as INTPTR, 0) };
    }

    // Wait for A9#0 to publish, then read the data. The acquire load pairs with
    // A9#0's release store, so the data read below cannot be satisfied before
    // the flag is seen.
    while shared.ready.load(Ordering::Acquire) == 0 {
        core::hint::spin_loop();
    }
    if shared.data.load(Ordering::Relaxed) == 0xdead_beef {
        post("A9#1 read shared data from A9#0\n\r");
        shared.ack.store(1, Ordering::Release);
    } else {
        post("A9#1 read unexpected shared data\n\r");
    }

    let mut counter: u32 = 0;
    loop {
        unsafe { sleep(3) };
        post("Hello from A9#1!\n\r");
        counter = counter.wrapping_add(1);
        // Keep the counter alive so it can be watched in the debugger
        black_box(counter);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    post("A9#1 panicked\r\n");
    loop {}
}
