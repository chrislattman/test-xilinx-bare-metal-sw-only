/*
 * shared_mem.h: memory map and handshake shared by A9#0 (hello_world) and A9#1 (hello_world_1)
 *
 * DDR layout (MMU sections are 1 MB, so each region is 1 MB aligned):
 *   0x00100000 - 0x0FEFFFFF  A9#0 private (hello_world lscript.ld)
 *   0x0FF00000 - 0x0FFFFFFF  shared between both cores (this struct)
 *   0x10000000 - 0x1FFFFFFF  A9#1 private (hello_world_1 lscript.ld)
 *
 * DDR is mapped Normal, write-back, Shareable on both cores and both have the
 * ACTLR SMP bit set, so the SCU keeps the L1 caches coherent. No cache
 * flush/invalidate is needed between cores, only ordering (release/acquire).
 *
 * The cores don't share peripherals: A9#0 owns UART0, so A9#1 doesn't print.
 * It posts its messages to the mailbox below (msg_post()) instead, and A9#0
 * prints them (msg_pop()).
 */

#ifndef SHARED_MEM_H
#define SHARED_MEM_H

#include <stdatomic.h>
#include <stdint.h>
#include <string.h>

#include <xil_mmu.h>
#include <xil_types.h>

#define SECTION_SIZE        0x100000U

#define CORE0_PRIVATE_BASE  0x00100000U
#define CORE0_PRIVATE_END   0x0FF00000U
#define SHARED_MEM_BASE     0x0FF00000U
#define CORE1_PRIVATE_BASE  0x10000000U
#define CORE1_PRIVATE_END   0x20000000U

/* Mailbox from A9#1 to A9#0: MSG_SLOTS messages of up to MSG_LEN - 1 characters */
#define MSG_SLOTS           8U
#define MSG_LEN             64U

typedef struct {
    // Written by A9#0, read by A9#1 once ready is set
    uint32_t data;
    // 0 = not ready, 1 = data published by A9#0
    atomic_uint ready;
    // 0 = not yet, 1 = data consumed by A9#1
    atomic_uint ack;
    // Single-producer (A9#1), single-consumer (A9#0) ring buffer of messages.
    // Both counters only increase (wrapping); msg_head - msg_tail = messages queued
    atomic_uint msg_head;   // written by A9#1: messages posted
    atomic_uint msg_tail;   // written by A9#0: messages printed
    char msgs[MSG_SLOTS][MSG_LEN];
} shared_t;

#define SHARED ((shared_t *)SHARED_MEM_BASE)

/*
 * A9#1 only: queues a message for A9#0 to print, truncated to MSG_LEN - 1
 * characters. Waits while the mailbox is full.
 */
static inline void msg_post(const char *msg)
{
    unsigned head = atomic_load_explicit(&SHARED->msg_head, memory_order_relaxed);
    // Acquire: A9#0 has finished reading the slot before it's overwritten
    while (head - atomic_load_explicit(&SHARED->msg_tail, memory_order_acquire) >= MSG_SLOTS)
        ;
    char *slot = SHARED->msgs[head % MSG_SLOTS];
    strncpy(slot, msg, MSG_LEN - 1);
    slot[MSG_LEN - 1] = '\0';
    // Release: the message is written before A9#0 can see the new head
    atomic_store_explicit(&SHARED->msg_head, head + 1, memory_order_release);
}

/*
 * A9#0 only: copies the oldest queued message into out and returns 1, or
 * returns 0 if the mailbox is empty.
 */
static inline int msg_pop(char out[MSG_LEN])
{
    unsigned tail = atomic_load_explicit(&SHARED->msg_tail, memory_order_relaxed);
    if (atomic_load_explicit(&SHARED->msg_head, memory_order_acquire) == tail)
        return 0;
    memcpy(out, SHARED->msgs[tail % MSG_SLOTS], MSG_LEN);
    // Release: done reading the slot before A9#1 can reuse it
    atomic_store_explicit(&SHARED->msg_tail, tail + 1, memory_order_release);
    return 1;
}

/*
 * Marks [base, end) as invalid in this core's MMU table, so any access from
 * this core raises a data abort. Invalid (fault) descriptors are honored even
 * though boot.S puts every domain in manager mode, which ignores AP bits.
 */
static inline void unmap_sections(UINTPTR base, UINTPTR end)
{
    for (UINTPTR addr = base; addr < end; addr += SECTION_SIZE) {
        Xil_SetTlbAttributes(addr, 0);
    }
}

#endif
