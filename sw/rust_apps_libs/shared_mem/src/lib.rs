//! Rust version of `shared_mem/shared_mem.h`: the memory map and handshake shared by
//! `hello-world-rs` (A9#0) and `hello-world-1-rs` (A9#1).
//!
//! DDR layout (MMU sections are 1 MB, so each region is 1 MB aligned):
//!
//! | Range                     | Owner                                         |
//! |---------------------------|-----------------------------------------------|
//! | `0x00100000 - 0x0FEFFFFF` | A9#0 private (`hello_world_rs/linker.ld`)     |
//! | `0x0FF00000 - 0x0FFFFFFF` | shared between both cores ([`Shared`])        |
//! | `0x10000000 - 0x1FFFFFFF` | A9#1 private (`hello_world_1_rs/linker.ld`)   |
//!
//! DDR is mapped Normal, write-back, Shareable on both cores and both have the
//! ACTLR SMP bit set, so the SCU keeps the L1 caches coherent. No cache
//! flush/invalidate is needed between cores, only ordering (release/acquire).
//!
//! The cores don't share peripherals: A9#0 owns UART0, so A9#1 doesn't print.
//! It posts its messages to the mailbox ([`Shared::post`]) instead, and A9#0
//! prints them ([`Shared::pop`]).
#![no_std]

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::{AtomicU32, Ordering};

pub const SECTION_SIZE: usize = 0x10_0000;

pub const CORE0_PRIVATE_BASE: usize = 0x0010_0000;
pub const CORE0_PRIVATE_END: usize = 0x0FF0_0000;
pub const SHARED_MEM_BASE: usize = 0x0FF0_0000;
pub const CORE1_PRIVATE_BASE: usize = 0x1000_0000;
pub const CORE1_PRIVATE_END: usize = 0x2000_0000;

/// Mailbox capacity: messages queued at once
pub const MSG_SLOTS: usize = 8;
/// Mailbox slot size: messages are truncated to `MSG_LEN - 1` bytes (plus a NUL,
/// like the C version)
pub const MSG_LEN: usize = 64;

/// The struct at [`SHARED_MEM_BASE`]. Same layout as `shared_t` in
/// `shared_mem.h`, so a Rust core can talk to a C core.
#[repr(C)]
pub struct Shared {
    /// Written by A9#0, read by A9#1 once `ready` is set. Atomic (accessed
    /// `Relaxed`) only so both cores can share it soundly; ordering comes from
    /// `ready`. The C version is a plain `uint32_t`.
    pub data: AtomicU32,
    /// 0 = not ready, 1 = data published by A9#0
    pub ready: AtomicU32,
    /// 0 = not yet, 1 = data consumed by A9#1
    pub ack: AtomicU32,
    /// Mailbox from A9#1 to A9#0, a single-producer single-consumer ring
    /// buffer. Both counters only increase (wrapping); `msg_head - msg_tail`
    /// messages are queued. Messages posted by A9#1 so far
    pub msg_head: AtomicU32,
    /// Messages printed by A9#0 so far
    pub msg_tail: AtomicU32,
    msgs: UnsafeCell<[[u8; MSG_LEN]; MSG_SLOTS]>,
}

const _: () = assert!(size_of::<Shared>() == 20 + MSG_SLOTS * MSG_LEN);

// SAFETY: the atomics are Sync; msgs is only accessed through post()/pop(), whose
// callers guarantee a single producer and a single consumer
unsafe impl Sync for Shared {}

impl Shared {
    /// Queues a message for A9#0 to print, truncated to `MSG_LEN - 1` bytes.
    /// Waits while the mailbox is full.
    ///
    /// # Safety
    /// Only one core (A9#1) may post, and A9#0 must have cleared `msg_head`
    /// and `msg_tail` before waking it.
    pub unsafe fn post(&self, msg: &[u8]) {
        let head = self.msg_head.load(Ordering::Relaxed);
        // Acquire: A9#0 has finished reading the slot before it's overwritten
        while head.wrapping_sub(self.msg_tail.load(Ordering::Acquire)) >= MSG_SLOTS as u32 {
            spin_loop();
        }
        let len = msg.len().min(MSG_LEN - 1);
        // SAFETY: A9#0 doesn't read this slot again until msg_head moves past it
        let slot = unsafe { &mut (*self.msgs.get())[head as usize % MSG_SLOTS] };
        slot[..len].copy_from_slice(&msg[..len]);
        slot[len] = 0;
        // Release: the message is written before A9#0 can see the new head
        self.msg_head.store(head.wrapping_add(1), Ordering::Release);
    }

    /// Copies the oldest queued message into `out` and returns its length, or
    /// returns `None` if the mailbox is empty.
    ///
    /// # Safety
    /// Only one core (A9#0) may pop.
    pub unsafe fn pop(&self, out: &mut [u8; MSG_LEN]) -> Option<usize> {
        let tail = self.msg_tail.load(Ordering::Relaxed);
        if self.msg_head.load(Ordering::Acquire) == tail {
            return None;
        }
        // SAFETY: A9#1 doesn't write this slot again until msg_tail moves past it
        *out = unsafe { (*self.msgs.get())[tail as usize % MSG_SLOTS] };
        // Release: done reading the slot before A9#1 can reuse it
        self.msg_tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(out.iter().position(|&b| b == 0).unwrap_or(MSG_LEN))
    }
}

/// The shared handshake struct. It starts out holding whatever DDR contained
/// at power-up, so A9#0 clears the flags before waking A9#1.
pub fn shared() -> &'static Shared {
    // SAFETY: the shared section is mapped on both cores and no linker script
    // places anything there. Every field is an atomic (interior mutability),
    // and any bit pattern is a valid u32.
    unsafe { &*(SHARED_MEM_BASE as *const Shared) }
}

/// Start address of every MMU section in `[base, end)`.
///
/// Used to unmap the other core's private DDR, e.g.
/// `for addr in sections(CORE1_PRIVATE_BASE, CORE1_PRIVATE_END) { Xil_SetTlbAttributes(addr as _, 0) }`.
/// Invalid (fault) descriptors are honored even though boot.S puts every
/// domain in manager mode, which ignores AP bits.
pub fn sections(base: usize, end: usize) -> impl Iterator<Item = usize> {
    (base..end).step_by(SECTION_SIZE)
}
